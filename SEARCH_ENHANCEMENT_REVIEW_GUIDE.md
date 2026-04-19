# Search Enhancement 变更说明与 Review 指南

本文档面向本次 `search-enhancement` 相关改动的 reviewer。目标不是重复源码，而是用"整体架构 + 关键链路 + 关键代码 + review 重点"的方式，帮助你快速恢复心智模型并进入有效 review。

信息来源：代码实现 `src/`、`migration/`。所有内容基于实际代码，不做杜撰。

## 1. 变更概览

这次改动给原先以 PostgreSQL 为唯一读模型的系统，增加了第二套读模型：

- **主数据源**: PostgreSQL（不变）
- **检索读模型**: MeiliSearch
- **语义向量**: vLLM embedding 服务
- **搜索三级退化**:
  - `hybrid`：关键词 + 向量并发召回，RRF 融合
  - `keyword_only`：MeiliSearch 仅关键词检索
  - `sql_fallback`：MeiliSearch 不可用时回退到 PostgreSQL `LIKE`

为了让检索索引与主库最终一致，新增了应用层 outbox + tombstone 机制：

- `search_sync_events` — 持久化索引同步事件
- `search_document_versions` — 记录每个实体的最新版本和删除状态

## 2. 系统全局架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                          HTTP Layer                                  │
│                                                                      │
│   GET /cards/search ─────┐                                           │
│                          ▼                                           │
│   ┌──────────────────────────────────────────┐                      │
│   │           AppState                        │                      │
│   │  ┌─────────────┐  ┌──────────────────┐   │                      │
│   │  │ luna_service │  │ search_service   │   │                      │
│   │  │ (LunaService │  │ (SearchService   │   │                      │
│   │  │  Trait)      │  │  Trait)          │   │                      │
│   │  └──────┬───────┘  └────────┬─────────┘   │                      │
│   └─────────┼───────────────────┼─────────────┘                      │
│             │                    │                                    │
│             │                    │ meili_ready?                       │
│             │                    ├──── YES ──► MeiliSearch            │
│             │                    │            (keyword / hybrid)      │
│             │                    └──── NO ───► SQL fallback           │
│             │                         │                               │
│             ▼                         ▼                               │
│   ┌─────────────────────────────────────────┐                        │
│   │         PostgreSQL (主库)                │                        │
│   └─────────────────────────────────────────┘                        │
│                                                                      │
│   同步链路（后台）:                                                    │
│   Luna CUD → outbox → IndexerService → MeiliSearch                   │
│                                                                      │
│   外部服务:                                                           │
│   ┌──────────────┐  ┌────────────────┐                               │
│   │ MeiliSearch  │  │ vLLM embedding  │                              │
│   │ (索引+检索)   │  │ (向量生成)      │                               │
│   └──────────────┘  └────────────────┘                               │
└─────────────────────────────────────────────────────────────────────┘
```

上图展示了系统的读写分离架构。**读链路**中，`SearchService` 作为统一入口，根据 `meili_ready` 标志决定走 MeiliSearch 还是 SQL fallback，两种路径最终都返回 `SearchResponse`。**写链路**中，Luna 域的业务 CUD 操作在同一事务内将变更事件写入 outbox 表，后台 IndexerService 独立轮询并消费事件，将变更推送到 MeiliSearch 索引。读写之间没有直接的函数调用依赖，唯一的耦合点是共享的 `meili_ready` 原子标志。外部的 vLLM 服务仅在 record 文档需要语义向量时参与，不影响基础的写入和关键词检索。

## 3. 读链路：统一搜索

```
HTTP GET /cards/search?q=xxx&entity_types=record,idol&director=John
       │
       ▼
┌──────────────────────────────────────────┐
│ search_handler::search()                 │
│  ├── Query<SearchParams> 解析参数         │
│  ├── Extension<Claims> 提取 JWT          │
│  ├── get_user_permission(claims.sub)      │
│  └── search_service.search(query, perm)   │
└──────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────┐
│ SearchService::search()                  │
│  ├── 规范化 limit/offset                  │
│  ├── 空 q → ValidationError              │
│  ├── 构建 permission filter              │
│  ├── 解析 entity_types                   │
│  ├── 构建 additional_filters             │
│  │   (director/studio/label/genre/date)  │
│  └── 拼接 filter_str                     │
└──────────────────────────────────────────┘
       │
       ├── meili_ready == true?
       │    YES ↓                    NO ↓
       │  ┌──────────────┐    ┌──────────────────┐
       │  │ search_meili │    │ sql_fallback      │
       │  │ 详见 §3.1    │    │ 详见 §3.3         │
       │  └──────┬───────┘    └────────┬──────────┘
       │         │                     │
       │    Ok → return          return (always)
       │    Err → fall through
       │         ↓
       │  ┌──────────────────┐
       │  │ sql_fallback     │
       │  └──────────────────┘
       │
       ▼
  SearchResponse { search_mode, total, limit, offset, results }
```

### 3.1 MeiliSearch 查询分流 (hybrid vs keyword_only)

文件: `src/domains/search/infra/impl_service/search_service_impl.rs` `search_meili()` 方法

```
search_meili(query, entity_types, filter_str, limit, offset)
       │
       ├── embedding_available?
       │    │
       │    ├── YES (hybrid 模式):
       │    │    fetch_limit = (offset + limit) × 3   ← 预取 3x 用于 RRF
       │    │    ┌───────────────────────────────────────┐
       │    │    │ tokio::join!                           │
       │    │    │  ├── keyword_search(fetch_limit, 0)   │ ← 并发
       │    │    │  └── embedding_service.embed(query)    │ ← 并发
       │    │    └───────────────────────────────────────┘
       │    │         │
       │    │    embedding 成功?
       │    │    ├── YES → vector_search(vec, fetch_limit, 0)
       │    │    └── NO  → vector_hits = []
       │    │         │
       │    │    has_vector_results?
       │    │    ├── YES → rrf_fusion(keyword, vector, K=60, fetch_limit)
       │    │    │         → 切片 skip(offset).take(limit)
       │    │    │         → search_mode = "hybrid"
       │    │    └── NO  → keyword hits 直接切片
       │    │              → search_mode = "keyword_only"
       │    │
       │    └── NO (keyword_only 模式):
       │         keyword_search(limit, offset)   ← Meili 原生分页
       │         → search_mode = "keyword_only"
       │
       ▼
  Ok(SearchResponse)
```

**为什么 hybrid 模式从 offset=0 预取 3x？**

RRF 融合需要看到两条链路的完整排名才能公平打分。如果只取 `limit` 条，两个列表中重叠的文档可能因为排名靠后而被截断，导致融合后排名不准确。预取 3x 是一个经验倍率，确保重叠文档有足够机会进入最终 top-N。

### 3.2 RRF 融合算法

文件: `src/domains/search/infra/impl_service/rrf.rs`

```
输入: keyword_hits = [A, B, C]  vector_hits = [B, D]

对每个 hit 计算 RRF 分数:
  score(doc) = Σ  1 / (k + rank + 1)     (k = 60)

keyword 路径:
  A (rank=0) → 1/(60+0+1) = 0.01639
  B (rank=1) → 1/(60+1+1) = 0.01613
  C (rank=2) → 1/(60+2+1) = 0.01587

vector 路径:
  B (rank=0) → 1/(60+0+1) = 0.01639
  D (rank=1) → 1/(60+1+1) = 0.01613

融合 (score = keyword_score + vector_score):
  A: 0.01639
  B: 0.01613 + 0.01639 = 0.03252  ← 出现在两个列表，分数最高
  C: 0.01587
  D: 0.01613

排序: B > A > D > C
```

关键属性：
- 同时出现在两条路径的文档获得更高的融合分数
- `k` 越大，排名差异的影响越小（平滑效果）
- 平局时按 `doc_id` 字典序打破，保证确定性

### 3.3 SQL Fallback

文件: `src/domains/search/infra/impl_service/sql_fallback.rs`

当 MeiliSearch 不可用时，使用 PostgreSQL `LIKE` 查询实现兜底搜索：

```
search_sql_fallback(db, query, entity_types, filter_str, ...)
       │
       ├── 转义 SQL 通配符: % → \%, _ → \_, \ → \\
       ├── wants_all = entity_types.is_empty()
       ├── parse_filters(filter_str) → ParsedFilters
       │
       ├── wants("record")?
       │    ├── LIKE 匹配 record.title
       │    ├── 查 director/studio/label/series 名匹配 → 得到 ID 列表
       │    ├── 查 genre 名匹配 → 通过 record_genre 反查 record ID
       │    ├── 查 idol 名匹配 → 通过 idol_participation 反查 record ID
       │    ├── 合并为 OR 条件 + permission 过滤
       │    ├── 叠加 director/studio/label/genre/date 过滤
       │    ├── count + offset/limit 分页
       │    └── 如果 record_only → 提前返回
       │
       └── 命名实体 (director/studio/label/series/genre/idol)
            ├── 各自 LIKE 匹配 name
            ├── 计数加入 total
            ├── 合并结果，全局 offset/limit 切片
            └── search_mode = "sql_fallback"
```

**SQL fallback 没有的能力**（与 Meili 对比）：
- 无高亮 (`highlight = None`)
- 无相关度评分 (`score = None`)
- 无语义搜索
- 查询性能依赖 PostgreSQL，无法水平扩展

## 4. 写链路：主库变更驱动索引同步

### 4.1 整体写链路

```
┌─────────────────────────────────────────────────────────────────┐
│                     Luna Service 写路径                          │
│                                                                  │
│  HTTP POST/PUT/DELETE /cards/...                                 │
│       │                                                          │
│       ▼                                                          │
│  ┌──────────────────────────────────────┐                       │
│  │ txn = db.begin()                     │                       │
│  │                                      │                       │
│  │  ① repo.create/update/delete(&txn)   │ ← 业务数据写入         │
│  │  ② outbox 写入 (&txn)                │ ← 同一事务             │
│  │  ③ tombstone 写入 (&txn)             │ ← 同一事务             │
│  │  ④ fan-out 写入 (&txn) (命名实体)    │ ← 同一事务             │
│  │                                      │                       │
│  │ txn.commit()                         │ ← 原子提交             │
│  └──────────────────────────────────────┘                       │
│       │                                                          │
│       │ (异步边界：commit 后，indexer 轮询发现新行)                │
│       ▼                                                          │
│  ┌──────────────────────────────────────┐                       │
│  │ search_sync_events 表                 │                       │
│  │  ┌────┬─────────────┬──────┬───────┐ │                       │
│  │  │ id │ entity_type │ event│ version│ │                       │
│  │  │  1 │ record      │ upsert│ ts1  │ │                       │
│  │  │  2 │ director    │ upsert│ ts2  │ │  ← indexer 轮询       │
│  │  │  3 │ record      │ upsert│ 0    │ │    (每 1 秒)           │
│  │  └────┴─────────────┴──────┴───────┘ │                       │
│  └──────────────────────────────────────┘                       │
│       │                                                          │
│       ▼                                                          │
│  ┌──────────────────────────────────────┐                       │
│  │ IndexerService (后台线程)             │                       │
│  │  claim_pending → process_event       │                       │
│  │  → upsert/delete MeiliSearch         │                       │
│  │  → mark_processed                    │                       │
│  └──────────────────────────────────────┘                       │
│       │                                                          │
│       ▼                                                          │
│  ┌──────────────────┐                                            │
│  │ MeiliSearch 索引  │ ← 最终一致                                  │
│  └──────────────────┘                                            │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Outbox 写入的三个辅助函数

文件: `src/domains/luna/infra/search_outbox.rs`

```
┌──────────────────────────────────────────────────────────────────┐
│ outbox_entity_upsert(&txn, entity_type, entity_id, name, affected)│
│  │                                                                │
│  ├── version = Utc::now().timestamp_nanos()                      │
│  ├── OutboxRepo::insert_event(&txn, type, id, "upsert", version, │
│  │                            payload={"name":"..."}, affected)   │
│  └── TombstoneRepo::upsert_version(&txn, type, id, version)      │
│      → INSERT ... ON CONFLICT DO UPDATE                           │
│        SET last_version = GREATEST(old, new), is_deleted = FALSE  │
├──────────────────────────────────────────────────────────────────┤
│ outbox_entity_delete(&txn, entity_type, entity_id, affected)      │
│  │                                                                │
│  ├── version = Utc::now().timestamp_nanos()                      │
│  ├── OutboxRepo::insert_event(&txn, type, id, "delete", version, │
│  │                            payload=None, affected)             │
│  └── TombstoneRepo::mark_deleted(&txn, type, id, version)        │
│      → INSERT ... ON CONFLICT DO UPDATE                           │
│        SET last_version = GREATEST(old, new), is_deleted = TRUE   │
├──────────────────────────────────────────────────────────────────┤
│ outbox_fanout_records(&txn, record_ids)                           │
│  │                                                                │
│  └── for each record_id:                                         │
│      OutboxRepo::insert_event(&txn, "record", id, "upsert",      │
│                                version=0, None, None)             │
│      注意: version=0 → fan-out hint, 不推进 tombstone             │
└──────────────────────────────────────────────────────────────────┘
```

### 4.3 命名实体更新时的完整事件序列（以 director 为例）

文件: `src/domains/luna/infra/impl_service/director.rs`

```
update_director(id, payload)
  │
  ├── txn = db.begin()
  │
  ├── pre_affected = find_affected_record_ids(&txn, "director", id)
  │   └── SELECT id FROM record WHERE director_id = $id
  │
  ├── director = repo.update(&txn, id, payload)
  │
  ├── surviving_id != id?  (duplicate merge 检测)
  │   │
  │   ├── YES (合并场景):
  │   │   ├── outbox_entity_delete(&txn, "director", id, [])
  │   │   │   → INSERT search_sync_events (type=director, event=delete)
  │   │   │   → INSERT/UPDATE search_document_versions (is_deleted=TRUE)
  │   │   │
  │   │   ├── surviving_affected = find_affected_record_ids(surviving_id)
  │   │   ├── all_affected = pre_affected ∪ surviving_affected (去重)
  │   │   │
  │   │   ├── outbox_entity_upsert(&txn, "director", surviving_id, name, all_affected)
  │   │   │   → INSERT search_sync_events (type=director, event=upsert, payload={name})
  │   │   │   → INSERT/UPDATE search_document_versions (version=ts, is_deleted=FALSE)
  │   │   │
  │   │   └── outbox_fanout_records(&txn, &all_affected)
  │   │       → INSERT search_sync_events × N (type=record, event=upsert, version=0)
  │   │
  │   └── NO (普通更新):
  │       ├── outbox_entity_upsert(&txn, "director", id, name, pre_affected)
  │       └── outbox_fanout_records(&txn, &pre_affected)
  │
  └── txn.commit()   ← 所有操作原子提交
```

### 4.4 受影响记录的查询路由

```
find_affected_record_ids(entity_type, entity_id)
  │
  ├── "director" → SELECT id FROM record WHERE director_id = $id  (FK)
  ├── "studio"   → SELECT id FROM record WHERE studio_id = $id    (FK)
  ├── "label"    → SELECT id FROM record WHERE label_id = $id     (FK)
  ├── "series"   → SELECT id FROM record WHERE series_id = $id    (FK)
  ├── "genre"    → SELECT record_id FROM record_genre WHERE genre_id = $id  (junction)
  ├── "idol"     → SELECT record_id FROM idol_participation WHERE idol_id = $id  (junction)
  └── _          → Vec::new()
```

```
find_affected_record_ids(entity_type, entity_id)
  │
  ├── "director" → SELECT id FROM record WHERE director_id = $id  (FK)
  ├── "studio"   → SELECT id FROM record WHERE studio_id = $id    (FK)
  ├── "label"    → SELECT id FROM record WHERE label_id = $id     (FK)
  ├── "series"   → SELECT id FROM record WHERE series_id = $id    (FK)
  ├── "genre"    → SELECT record_id FROM record_genre WHERE genre_id = $id  (junction)
  ├── "idol"     → SELECT record_id FROM idol_participation WHERE idol_id = $id  (junction)
  └── _          → Vec::new()
```

这个查询路由在 `src/domains/luna/infra/search_outbox.rs` 中实现。命名实体和 record 之间存在两种关联方式：director/studio/label/series 通过 record 表上的 FK 列直接关联（一对一或一对多），因此直接 `SELECT id FROM record WHERE {column} = $id`；genre 和 idol 则通过中间表（`record_genre`、`idol_participation`）实现多对多关联，需要先从中间表反查 record_id。查询结果是一组 record ID，用于生成 fan-out 的 outbox 事件——当命名实体变更时，所有关联的 record 文档都需要重建。

## 5. 索引同步器：IndexerService

### 5.1 组件关系

```
SearchService (门面)
  ├── search_repo: Arc<MeiliSearchRepo>     ← 查询用
  ├── embedding_service: Arc<EmbeddingService> ← 向量生成
  ├── meili_ready: Arc<AtomicBool>          ← 查询分流开关
  └── indexer: Arc<IndexerService>          ← 后台同步
        ├── db: DatabaseConnection
        ├── search_repo: Arc<MeiliSearchRepo>
        ├── embedding_service: Arc<EmbeddingService>
        └── meili_ready: Arc<AtomicBool>    ← 共享同一个 flag
```

注意 `meili_ready` 是 `Arc<AtomicBool>`，被 SearchService 和 IndexerService 共享：
- IndexerService 写入（同步完成后置 true，Meili 不可用时置 false）
- SearchService 读取（决定查询走 Meili 还是 SQL fallback）

这个设计的关键点在于 `SearchService` 和 `IndexerService` 共享同一组 `Arc<MeiliSearchRepo>` 和 `Arc<EmbeddingService>` 实例——读和写操作使用相同的 MeiliSearch 连接和 embedding 客户端，避免了资源浪费。`IndexerService` 本身不实现 trait（与 `SearchServiceTrait` 不同），因为它是一个内部基础设施组件，没有多个实现的必要，也没有被外部注入或替换的需求。

### 5.2 启动同步状态机

文件: `src/domains/search/infra/indexer/indexer_service.rs` `run_startup_sync()`

```
trigger_startup_sync()
  │
  └── tokio::spawn ──► run_startup_sync()
                          │
                          ├── MeiliSearch health_check()  ── FAIL → return
                          │
                          ├── init_index() (创建索引 + 配置 searchable/filterable)
                          │
                          ├── embedding_service.check_health()
                          │
                          ├── get_document_count("")
                          │   │
                          │   ├── count == 0 (空索引):
                          │   │   └── run_full_sync()
                          │   │       ├── 同步所有 director/genre/label/studio/series/idol/record
                          │   │       ├── record 批量生成 embedding (如果 vLLM 可用)
                          │   │       ├── 更新所有 tombstone version
                          │   │       └── 用当前纳秒时间戳作为 sync_version
                          │   │
                          │   └── count > 0 (非空索引):
                          │       └── reconcile_counts()
                          │           ├── 对比 PG 和 Meili 各实体类型数量
                          │           ├── PG > Meili → full_sync 修复
                          │           └── Meili > PG → 删除 ghost documents
                          │
                          ├── drain pending outbox events
                          │   └── 循环 claim_pending + process_event 直到无新事件
                          │
                          └── remaining == 0?
                              ├── YES → meili_ready = true  ← Meili 可用了
                              └── NO  → meili_ready = false (留给 indexer loop 继续)
                                       │
                                       ▼
                                  run_indexer_loop()  ← 进入主循环
```

### 5.3 Indexer 主循环

文件: `src/domains/search/infra/indexer/indexer_service.rs` `run_indexer_loop()`

```
run_indexer_loop() — 每 POLL_INTERVAL_SECS (1s) 一次迭代

┌──────────────────────────────────────────────────────────┐
│ LOOP                                                      │
│  │                                                        │
│  ├── ① MeiliSearch health_check()                        │
│  │   └── FAIL → meili_ready = false, sleep(5s), continue │
│  │                                                        │
│  ├── ② full_sync_done?                                   │
│  │   └── NO → init_index + full_sync                     │
│  │                                                        │
│  ├── ③ embedding_service.check_health()                  │
│  │                                                        │
│  ├── ④ vLLM 恢复检测:                                    │
│  │   embedding_now_available && !embedding_was_available  │
│  │   → backfill_missing_vectors()                        │
│  │                                                        │
│  ├── ⑤ reclaim_expired_claims(LEASE_TIMEOUT=300s)        │
│  │   → UPDATE ... SET claimed_by=NULL WHERE expired       │
│  │                                                        │
│  ├── ⑥ claim_pending(worker_id, batch=50, lease=300s)    │
│  │   │                                                    │
│  │   ├── 有事件:                                          │
│  │   │   for each event:                                  │
│  │   │     process_event → Ok → mark_processed            │
│  │   │                   → Err → release_claim            │
│  │   │   └── 如果 meili_ready==false: count_pending       │
│  │   │       → 0 → meili_ready = true                    │
│  │   │                                                    │
│  │   └── 无事件:                                          │
│  │       如果 meili_ready==false: count_pending           │
│  │                                                        │
│  ├── ⑦ reconciliation_timer += 1                         │
│  │   >= 3600s? → reconcile_counts() + backfill            │
│  │                                                        │
│  └── sleep(1s)                                            │
└──────────────────────────────────────────────────────────┘
```

关键常量:

| 常量 | 值 | 含义 |
|------|-----|------|
| `POLL_INTERVAL_SECS` | 1 | 轮询间隔 |
| `CLAIM_BATCH_SIZE` | 50 | 每次最多 claim 事件数 |
| `LEASE_TIMEOUT_SECS` | 300 | claim 租约超时（5 分钟） |
| `RECONCILIATION_INTERVAL_SECS` | 3600 | 定期对账间隔（1 小时） |

### 5.4 Outbox Claim 机制

文件: `src/domains/search/infra/outbox_repo_impl.rs` `claim_pending()`

```sql
UPDATE search_sync_events
SET claimed_by = $1, claimed_at = NOW()
WHERE id IN (
    SELECT id FROM search_sync_events
    WHERE processed_at IS NULL
    AND (
        claimed_by IS NULL
        OR claimed_at < NOW() - INTERVAL '1 second' * $2   -- 租约过期
    )
    ORDER BY id ASC
    LIMIT $3
    FOR UPDATE SKIP LOCKED   -- 跳过被其他 worker 锁住的行
)
RETURNING id, entity_type, entity_id, event_type, entity_version,
          payload, affected_record_ids
```

支持多 worker 并发 claim：
- `SKIP LOCKED` 保证两个 worker 不会 claim 同一行
- `LEASE_TIMEOUT` 保证 worker 崩溃后事件不会永远卡住
- `reclaim_expired_claims()` 是兜底清理

支持此查询的索引：

| 索引名 | 列 | 用途 |
|--------|-----|------|
| `idx_search_sync_events_pending` | `(processed_at, claimed_by)` | 加速 `WHERE processed_at IS NULL` 子查询 |
| `idx_search_sync_events_claimed` | `(claimed_at)` | 加速 `reclaim_expired_claims` 和 `claimed_at < ...` 条件 |

## 6. Event Processing：事件消费细节

文件: `src/domains/search/infra/indexer/event_processor.rs`

### 6.1 Upsert 路径

```
process_upsert_event(event)
  │
  ├── TombstoneRepo::get_version(type, id)
  │   │
  │   ├── Some(version) 且 is_deleted == true → return Ok  (已删除，跳过)
  │   ├── Some(version) 且 event.version < version → return Ok  (过期事件，跳过)
  │   └── None 或 event.version >= version → 继续
  │
  ├── construct_document(event)  ← 构建 SearchDocument
  │   │
  │   ├── 命名实体 (非 record):
  │   │   → payload 中提取 name 作为 title
  │   │   → permission = 0
  │   │   → 其余字段全为 None
  │   │
  │   └── record:
  │       → 查 record 表获取基本字段
  │       → 查 director/studio/label/series 表获取名称
  │       → 查 record_genre + genre 表获取 genre_names
  │       → 查 idol_participation + idol 表获取 idol_names
  │       → permission = record.permission
  │
  ├── 如果 entity_type == "record" 且 embedding 可用:
  │   doc.vectors = wrap_vectors(embed(title))
  │   → {"default": [0.1, 0.2, ...]}
  │
  ├── search_repo.upsert_document(&doc) → MeiliSearch
  │
  └── event.version > 0?
      └── YES → TombstoneRepo::upsert_version(type, id, version)
               (version=0 的 fan-out 事件不推进 tombstone)
```

### 6.2 Delete 路径

```
process_delete_event(event)
  │
  ├── TombstoneRepo::get_version(type, id)
  │   └── event.version > 0 且 < last_version → return Ok (过期删除)
  │       注意: is_deleted==true 时不跳过，因为 tombstone 和 outbox 在同一事务
  │
  ├── doc_id = "{entity_type}__{entity_id}"
  ├── search_repo.delete_document(&doc_id) → MeiliSearch
  │
  └── TombstoneRepo::mark_deleted(type, id, version)
```

## 7. Full Sync 与 Reconciliation

### 7.1 Full Sync

文件: `src/domains/search/infra/indexer/full_sync.rs`

```
run_full_sync()
  │
  ├── sync_version = 当前纳秒时间戳
  │   (所有文档统一使用这个 version，保证旧 outbox 事件被拒绝)
  │
  ├── 逐个同步命名实体 (director/genre/label/studio/series/idol):
  │   ├── SELECT * FROM entity
  │   ├── 逐个 upsert_document → MeiliSearch
  │   └── upsert_version(type, id, sync_version) → tombstone
  │
  ├── 批量同步 record:
  │   ├── SELECT * FROM record
  │   ├── 批量加载 genre_names: record_genre + genre (1 query)
  │   ├── 批量加载 idol_names: idol_participation + idol (1 query)
  │   ├── 构建 director/studio/label/series 名映射 (内存 map)
  │   │
  │   ├── vLLM 可用? → embed_batch(titles) → 填充 vectors
  │   │
  │   ├── chunks(100) → batch_upsert → MeiliSearch
  │   └── 逐个 upsert_version → tombstone
  │
  └── 注意: 故意不做 stale 文档清理
      (因为 PG 快照和 Meili 快照不是原子一致的，可能误删新文档)
```

### 7.2 Reconciliation

文件: `src/domains/search/infra/indexer/reconciliation.rs`

```
reconcile_counts()
  │
  ├── 对比 record 数量: PG count vs Meili count
  │   ├── PG > Meili → run_full_sync() 修复
  │   └── Meili > PG → remove_ghost_documents()
  │       ├── get_entity_ids("record") → Meili 端所有 entity_id
  │       ├── fetch_pg_record_ids() → PG 端所有 record.id
  │       └── 差集 → 逐个 delete_document
  │
  ├── 对比每个命名实体数量: director/studio/label/series/genre/idol
  │   └── 同上逻辑
  │
  └── return true/false (是否全部一致)

backfill_missing_vectors()
  │
  ├── 分页 (offset, batch_size=50) 扫描 Meili record 文档
  ├── 客户端过滤: _vectors == null 的文档
  ├── embed_batch(titles) → 生成向量
  ├── 逐个 upsert_document (带 vectors) 回 Meili
  └── MAX_BACKFILL_ITERATIONS = 200 (约 10K 文档上限)
```

## 8. MeiliSearch 索引设计

### 8.1 统一索引文档

索引名: `luna_search`，文档类型 `SearchDocument`:

```
┌─────────────────────────────────────────────────────────┐
│ SearchDocument                                           │
│                                                          │
│  id (doc_id)     = "{entity_type}__{entity_id}"           │
│  title           = 显示标题                              │
│  entity_type     = "record" | "idol" | "director" | ...  │
│  entity_id       = 数据库主键 (string)                    │
│  entity_version  = 单调递增版本号                         │
│  permission      = record 用实际值，命名实体用 0           │
│  ──────────────── record 专用字段 ──────────────────      │
│  date            = 发布日期 (Option)                      │
│  duration        = 时长秒数 (Option)                      │
│  director_name   = 导演名 (Option)                        │
│  studio_name     = 片商名 (Option)                        │
│  label_name      = 厂牌名 (Option)                        │
│  series_name     = 系列名 (Option)                        │
│  genre_names     = 题材名列表 (Option<Vec>)                │
│  idol_names      = 演员名列表 (Option<Vec>)                │
│  ──────────────── 向量字段 ──────────────────────────     │
│  _vectors        = {"default": [0.1, 0.2, ...]}           │
│                   (仅 record, skip_serializing_if_none)   │
└─────────────────────────────────────────────────────────┘
```

### 8.2 索引配置

文件: `src/domains/search/infra/meilisearch/index_setup.rs`

**Searchable Attributes** (全文匹配字段):
```
title, entity_id, director_name, studio_name, label_name,
series_name, genre_names, idol_names
```

`entity_id` 被包含在 searchable attributes 中，因此用户可以通过 record ID（如 `ABF-050`）直接搜索到对应记录。

**Filterable Attributes** (过滤/排序字段):
```
entity_type, date, duration, permission,
director_name, studio_name, label_name,
series_name, genre_names, idol_names
```

**Embedder 配置** (通过 raw HTTP PATCH，SDK 0.28 不支持):
```json
{ "embedders": { "default": { "source": "userProvided" } } }
```

`userProvided` 意味着向量由应用层提供（通过 vLLM），MeiliSearch 不自动生成。

### 8.3 SDK + Raw HTTP 混合方案

```
┌──────────────────────────────────────────────────────┐
│ MeiliSearchRepo                                       │
│                                                       │
│  SDK (meilisearch-sdk 0.28):                         │
│   ├── init_index (create + settings)                 │
│   ├── upsert_document / batch_upsert                 │
│   ├── delete_document                                │
│   ├── keyword_search                                 │
│   └── get_document_count                             │
│                                                       │
│  Raw HTTP (reqwest):                                  │
│   ├── vector_search (POST /indexes/{name}/search)    │
│   │   → body: { vector, hybrid: { embedder } }       │
│   ├── find_records_missing_vectors                   │
│   │   → POST /indexes/{name}/documents/fetch         │
│   └── get_entity_ids                                 │
│       → POST /indexes/{name}/documents/fetch          │
└──────────────────────────────────────────────────────┘
```

`MeiliSearchRepo` 同时使用 SDK 和原始 HTTP 的原因是 SDK 版本限制。当前使用的 `meilisearch-sdk 0.28` 尚未暴露 hybrid/vector search 参数（`vector`、`hybrid.embedder`）和文档批量获取 API（`POST /indexes/{name}/documents/fetch`），因此这些操作通过 `reqwest::Client` 直接构造 HTTP 请求实现。SDK 覆盖的操作（CRUD、关键词搜索、统计）保持使用 SDK 以获得类型安全和自动重试。两个客户端（SDK 内部的 HTTP 客户端和手动创建的 `reqwest::Client`）共享同一组 MeiliSearch 凭证，但连接池各自独立。

### 8.4 数据类型语义与生成规则

搜索链路中有一组贯穿 outbox → indexer → MeiliSearch 的核心标识字段。理解它们的含义和生成方式，是理解整个同步机制的基础。

#### `entity_type` — 实体类型标签

```
entity_type 的取值由业务域决定，固定为以下 7 种字符串字面量：

  "record"   ← 影片/记录，主实体
  "director" ← 导演
  "studio"   ← 片商
  "label"    ← 厂牌
  "series"   ← 系列
  "genre"    ← 题材
  "idol"     ← 演员

来源: src/domains/luna/infra/search_outbox.rs 中的调用者硬编码传入
  outbox_entity_upsert(&txn, "director", id, name, affected)
                       ^^^^^^^^^^^^^^^^
```

这些值不是枚举类型序列化的结果，而是各 service 实现中直接传入的字面量字符串。它们同时用作 outbox 表的 `entity_type` 列值、tombstone 表的复合主键一部分、以及 MeiliSearch 文档的 `entity_type` 字段（用于 filter 查询 `entity_type = "record"`）。

#### `entity_id` — 实体主键

```
entity_id 的格式取决于实体类型：

  record   → 使用原始的 String 类型主键 (record.id)
             例: "ABC-123"
  其他实体  → 将 i64 主键转为字符串 (id.to_string())
             例: "42"

来源:
  // record (src/domains/luna/infra/impl_service/record.rs)
  outbox_entity_upsert(&txn, "record", &record.id, ...)  // String 直接用
  // director (src/domains/luna/infra/impl_service/director.rs)
  outbox_entity_upsert(&txn, "director", &id.to_string(), ...)  // i64 → String
```

命名实体的主键是 `i64`，通过 `.to_string()` 转为字符串存入 outbox 和 tombstone 表。record 的主键本身就是 `String` 类型（可能是业务编码），直接传入不做转换。

#### `doc_id` — MeiliSearch 文档 ID

```
doc_id 格式: "{entity_type}__{entity_id}"

例:
  "record__ABC-123"
  "director__42"
  "idol__7"

来源: src/domains/search/infra/indexer/event_processor.rs
  let doc_id = format!("{}:{}", event.entity_type, event.entity_id);
```

由于 MeiliSearch 使用统一索引 `luna_search` 存放所有实体类型的文档，不同实体可能有相同的主键值（比如 record 的 "42" 和 director 的 "42"）。通过 `{entity_type}:` 前缀确保 doc_id 在索引内全局唯一。这个 doc_id 同时用于文档的 upsert（覆盖写）和 delete。

#### `entity_version` — 单调递增版本号

```
版本号生成规则（三种场景）:

1. 正常 CUD 事件 (outbox 写入时):
   version = Utc::now().timestamp_nanos()
   例: 1744800000000000000

   来源: src/domains/luna/infra/search_outbox.rs
     let version = Utc::now().timestamp_nanos();

2. Fan-out 事件 (命名实体变更触发关联 record 重建):
   version = 0

   来源: src/domains/luna/infra/search_outbox.rs
     OutboxRepo::insert_event(&txn, "record", id, "upsert",
                              0,    // ← 固定为 0
                              None, None)

   为什么是 0?
   fan-out 是"提示"而非"权威"——它告诉 indexer "这个 record 需要重建"，
   但如果同时有对同一 record 的真实编辑（version=timestamp），真实编辑
   优先。tombstone 只在 version > 0 时才推进，所以 version=0 永远不会
   覆盖一个已有的正版本号。

3. Full sync (启动时全量同步):
   sync_version = 当前纳秒时间戳 (单一值)
   所有文档统一使用这个 version

   来源: src/domains/search/infra/indexer/full_sync.rs
     let sync_version = Utc::now().timestamp_nanos();
     // 对所有实体: upsert_version(type, id, sync_version)

   统一 version 的意义: 全量同步完成后，所有 outbox 中
   version < sync_version 的旧事件都会被 tombstone 拒绝，
   避免已同步的文档被过时事件覆盖。
```

版本号的核心不变量由 tombstone 表的 `GREATEST(old, new)` 保证——无论并发事务以何种顺序提交，`last_version` 只会单调递增，不会倒退。indexer 在处理事件时，会先查询 tombstone 检查 `event.version < last_version`，过时的事件直接跳过。

#### `permission` — 权限等级

```
permission 赋值规则:

  record 文档  → 使用 record 表中的 permission 列值 (i32)
                 例: 0, 50, 100, 2147483647

  命名实体文档 → 固定为 0 (所有人均可见)

来源: src/domains/search/infra/indexer/event_processor.rs
  construct_document() 中:
    // 命名实体:
    doc.permission = 0;
    // record:
    doc.permission = record_model.permission;

查询时过滤:
  filter_str 中包含 "permission <= {user_permission}"
  → 只有 permission 值 <= 用户等级的 record 会被返回
```

命名实体的 `permission = 0` 意味着所有用户（包括未认证用户）都能通过搜索找到导演、片商等信息。record 使用实际的 permission 值，实现了分级可见性。由于命名实体本身不包含敏感信息，这种设计是合理的。

## 9. Filter 构建管线

文件: `src/domains/search/infra/impl_service/search_service_impl.rs` + `filter_utils.rs`

```
SearchQuery { q, director, studio, ... }
       │
       ▼
构建 additional_filters (Vec<String>):
  ├── permission_filter = "permission <= {user_permission}"
  ├── director? → "director_name = \"{escaped}\""
  ├── studio?   → "studio_name = \"{escaped}\""
  ├── label?    → "label_name = \"{escaped}\""
  ├── genre?    → "genre_names = \"{escaped}\""
  ├── date_from? → "date >= \"{escaped}\""
  └── date_to?   → "date <= \"{escaped}\""
       │
       ▼
filter_str = additional_filters.join(" AND ")
  → "permission <= 2147483647 AND director_name = \"John\""
       │
       ▼ (传入 MeiliSearch)
build_filter_string(entity_types, filter_str)
  ├── entity_types 非空? → "(entity_type = \"record\" OR entity_type = \"idol\")"
  └── 合并: "(entity_type = ...) AND {filter_str}"
       │
       ▼
  SQL Fallback 侧:
  parse_filters(filter_str) → ParsedFilters { director, studio, ... }
  → 用于构建各实体的 SQL WHERE 条件
```

`escape_filter_value`: 替换 `\` → `\\` 和 `"` → `\"`，防止 filter injection。

`split_filter_clauses`: 在 ` AND ` 上分割，但跳过引号内的 `AND`（如 `"Rock AND Roll"`）。

## 10. 两张新增表

### 10.1 `search_sync_events` (outbox 表)

文件: `migration/src/m20260408_000001_create_search_tables.rs`

```
┌──────────────────────────────────────────────────────────────┐
│ search_sync_events                                           │
│                                                               │
│  id                 BIGINT PK AUTO_INCREMENT                  │
│  entity_type        VARCHAR(32) NOT NULL                      │
│  entity_id          VARCHAR(255) NOT NULL                     │
│  event_type         VARCHAR(16) NOT NULL  ("upsert"/"delete")│
│  entity_version     BIGINT NOT NULL DEFAULT 0                 │
│  payload            JSON NULL  (如 {"name":"..."})            │
│  affected_record_ids JSON NULL  (如 ["rec1","rec2"])         │
│  created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()        │
│  processed_at       TIMESTAMPTZ NULL  (NULL=未消费)           │
│  claimed_by         VARCHAR(64) NULL   (worker ID)           │
│  claimed_at         TIMESTAMPTZ NULL   (claim 时间)           │
│                                                               │
│  INDEX (processed_at, claimed_by) — pending 事件查询          │
│  INDEX (claimed_at)              — 过期 claim 回收            │
└──────────────────────────────────────────────────────────────┘
```

### 10.2 `search_document_versions` (tombstone 表)

```
┌──────────────────────────────────────────────────────────────┐
│ search_document_versions                                     │
│                                                               │
│  entity_type    VARCHAR(32) PK                                │
│  entity_id      VARCHAR(255) PK                               │
│  last_version   BIGINT NOT NULL DEFAULT 0                     │
│  is_deleted     BOOLEAN NOT NULL DEFAULT FALSE                 │
│  updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()             │
│                                                               │
│  写入方式: INSERT ... ON CONFLICT DO UPDATE                    │
│    last_version = GREATEST(old, new)   ← 只升不降             │
└──────────────────────────────────────────────────────────────┘
```

GREATEST 保证：即使并发事务以不同顺序提交，version 也只会前进，不会倒退。

## 11. Embedding 服务状态机

文件: `src/domains/search/infra/embedding/embedding_service.rs`

```
           check_health() 成功
    UNAVAILABLE ──────────────────► AVAILABLE
         ▲                           │
         │  embed() 失败              │ embed() 正常
         │  或 check_health() 失败    │
         ◄───────────────────────────┘

状态转换:
  初始 → UNAVAILABLE
  check_health() → true  → AVAILABLE
  check_health() → false → UNAVAILABLE
  embed() → Err → UNAVAILABLE
  embed() → None (UNAVAILABLE 时直接返回) → 不改变状态
```

关键行为：
- UNAVAILABLE 时 `embed()` 直接返回 `None`，不发送请求
- `embed()` 失败自动标记为 UNAVAILABLE
- `check_health()` 在 indexer loop 每个迭代中被调用，实现自动恢复

这个状态机决定了搜索的降级行为。当 vLLM 处于 `UNAVAILABLE` 状态时，新索引的 record 文档不会携带向量（`_vectors = null`），搜索自动退化为 `keyword_only` 模式。当 vLLM 恢复后，indexer 检测到状态转换，会触发 `backfill_missing_vectors()` 为已有文档补填向量（§7.2），此后新搜索请求才能使用 hybrid 模式。状态机的转换阈值很简单——一次 `check_health()` 成功即切换到 `AVAILABLE`，一次 `embed()` 失败即切换回 `UNAVAILABLE`——这种快速切换策略适合 embedding 服务作为可选增强组件的定位。

## 12. `meili_ready` 的完整生命周期

```
应用启动
  │
  ▼
meili_ready = false  ← 初始值
  │
  ▼
run_startup_sync()
  ├── Meili 不可用 → return (保持 false)
  ├── full_sync 完成 + backlog 清空 → meili_ready = true
  └── backlog 未清空 → 保持 false
  │
  ▼
run_indexer_loop()
  ├── Meili health_check 失败 → meili_ready = false
  ├── backlog drain 完成 → meili_ready = true
  └── 循环往复
  │
  ▼
SearchService::search()
  ├── meili_ready == true → 尝试 Meili，失败 → SQL fallback
  └── meili_ready == false → 直接 SQL fallback
```

`meili_ready` 不等于 "MeiliSearch 进程活着"，它的含义更保守：
- 索引已初始化
- 启动同步已完成
- backlog 已处理到安全状态

## 13. 与应用接线相关的变化

### 13.1 search 作为独立 domain

```rust
// src/domains.rs
pub mod auth;
pub mod device;
pub mod file;
pub mod luna;
pub mod search;    // ← 新增
pub mod user;
```

路由挂载：

```rust
// src/app.rs
.nest("/cards", luna_routes().merge(search_routes()))
```

- `search_routes()` 挂在 `/cards` 下 → 完整路径 `GET /cards/search`
- 与所有 `/cards/*` 一样受 JWT 保护

### 13.2 AppState 新增 search_service

```rust
pub struct AppState {
    pub config: Config,
    pub auth_service: Arc<dyn AuthServiceTrait>,
    pub user_service: Arc<dyn UserServiceTrait>,
    pub device_service: Arc<dyn DeviceServiceTrait>,
    pub file_service: Arc<dyn FileServiceTrait>,
    pub luna_service: Arc<dyn LunaServiceTrait>,
    pub search_service: Arc<dyn SearchServiceTrait>,  // ← 新增
}
```

### 13.3 Bootstrap 装配

```rust
// src/common/bootstrap.rs
let search_service: Arc<dyn SearchServiceTrait> =
    SearchService::create_service(config.clone(), pool.clone());

// src/main.rs
state.search_service.trigger_startup_sync();  // 启动后台 indexer
```

`create_service()` 内部构造完整依赖链：
```
Config + DB
  → MeiliSearchClient (url, key, "luna_search")
  → EmbeddingClient (vllm_url, model, timeout)
  → MeiliSearchRepo (client)
  → EmbeddingService (client, batch_size)
  → IndexerService (db, config, repo, embedding, meili_ready)
  → SearchService (db, config, repo, embedding, meili_ready, indexer)
```

## 14. 类型分层

```
┌─────────────────────────────────────────────────────────────┐
│ entities/ (SeaORM 表映射)                                    │
│  search_sync_events::Model   → search_sync_events 表        │
│  search_document_versions::Model → search_document_versions 表│
│  record::Model, director::Model, ...  → 业务表              │
│  用途: 纯数据库行映射，关心列类型和 relation                    │
├─────────────────────────────────────────────────────────────┤
│ domain/model/ (业务模型)                                     │
│  SearchDocument → 搜索索引文档                                │
│  SyncEventType  → upsert / delete 枚举                       │
│  SearchEntityType → record / idol / director / ... 枚举      │
│  用途: 系统内部如何看待对象，不等于表结构也不等于 HTTP 响应      │
├─────────────────────────────────────────────────────────────┤
│ dto/ (API 边界)                                              │
│  SearchQuery / SearchParams → 请求参数                       │
│  SearchResponse / SearchResultItem → 响应结构                │
│  用途: 关心序列化、校验、Swagger schema                       │
├─────────────────────────────────────────────────────────────┤
│ repository trait 内的契约类型                                 │
│  OutboxEvent → claim 返回的事件数据                           │
│  DocumentVersion → tombstone 查询结果                        │
│  KeywordSearchResult / VectorSearchResult / SearchHit        │
│  ParsedFilters → filter 解析中间结果                         │
│  用途: 层间传递的中间数据，不直接落库也不直接返回给前端           │
└─────────────────────────────────────────────────────────────┘
```

这四层类型的转换关系贯穿整个搜索链路。**写入时**，业务操作生成 `entities/` 的行映射对象，通过 `search_outbox.rs` 转为 `OutboxEvent` 写入数据库，同时更新 `DocumentVersion`。**消费时**，IndexerService 从 `OutboxEvent` 重新构建业务数据（查询 `entities/`），组装为 `SearchDocument`（`domain/model/`），推送到 MeiliSearch。**查询时**，HTTP 请求先反序列化为 `dto/` 层的 `SearchParams`，经 `SearchService` 处理后通过 `SearchRepository` trait 的返回类型（`KeywordSearchResult`、`VectorSearchResult`）获得原始数据，再转换为 `dto/` 层的 `SearchResponse` 返回。注意 `SearchDocument` 是唯一同时被 domain 层和 MeiliSearch 共享的类型——它既在代码中作为业务模型存在，又直接 JSON 序列化为 MeiliSearch 的文档格式。

## 15. 当前权限模型

文件: `src/domains/search/infra/impl_service/search_service_impl.rs` `get_user_permission()`

```
get_user_permission(user_id)
  │
  ├── UsersEntity::find_by_id(user_id) → None → return 0
  │                                              (未认证/无效 token)
  └── 用户存在 → return i32::MAX
                     (当前所有已认证用户看到所有 record)
```

原因（代码注释）: `user_ext` 表的 `user_id` 类型是 `i64`，与 `users.id: String/UUID` 不匹配，且没有 migration 创建它。因此暂时返回 `i32::MAX`。

**实际效果**:
- 未认证用户: `permission = 0`，看不到受限 record
- 已认证用户: `permission = i32::MAX`，看到所有 record

## 16. 当前实现的边界与限制

| 限制 | 说明 | 修复机制 |
|------|------|---------|
| 直接改 DB 不触发同步 | 只有走应用层 service 才写 outbox | 定期 reconciliation (1 小时) |
| reconciliation 只检测数量漂移 | PG 和 Meili 文档数相同但内容不同时不检测 | outbox 机制保证内容正确性 |
| 索引更新有延迟 | 后台轮询，正常 ~1 秒 | 搜索场景可接受 |
| `update_record_links` 不写 outbox | 链接变更不触发搜索同步 | links 不在搜索字段中 |
| fan-out version=0 不推进 tombstone | 并发的真实 record 更新优先 | 设计如此，非缺陷 |

## 17. 建议的 Review 顺序

1. **接入层**: `src/app.rs` → `src/main.rs` → `src/common/app_state.rs` → `src/common/bootstrap.rs`
2. **接口层**: `src/domains/search/api/*` + `dto/search_dto.rs`
3. **查询链路**: `src/domains/search/infra/impl_service/*` (search_service → filter → rrf → sql_fallback)
4. **MeiliSearch**: `src/domains/search/infra/meilisearch/*` (repo → client → index_setup)
5. **表结构**: `migration/...create_search_tables.rs` + `src/entities/search_*`
6. **同步链路**: `src/domains/search/infra/indexer/*` + `outbox_repo_impl` + `tombstone_repo_impl`
7. **写链联动**: `src/domains/luna/infra/search_outbox.rs`
8. **Record 写链**: `src/domains/luna/infra/impl_service/record.rs`
9. **Fan-out 抽样**: `src/domains/luna/infra/impl_service/director.rs`
10. **批量加载**: `src/domains/luna/infra/impl_repository/record_loader.rs`

## 18. Review 检查清单

### 接口层
- [ ] `GET /cards/search` 受 JWT 保护
- [ ] 空 `q` 返回 400
- [ ] DTO 参数与 OpenSpec 一致

### 查询行为
- [ ] `entity_types` 过滤在 keyword / vector / SQL fallback 中都生效
- [ ] 字段过滤在 Meili 和 SQL fallback 中语义一致
- [ ] hybrid 分页预取 3x 是否足够
- [ ] RRF 融合对重叠文档的提权是否正确

### 索引同步
- [ ] 所有影响搜索结果的写路径都写了 outbox
- [ ] 事务失败时业务数据和 outbox 不会半成功
- [ ] stale event 被 tombstone 正确拒绝

### Fan-out
- [ ] 命名实体更新/删除后 record 文档一定会重建
- [ ] duplicate merge 正确处理 old delete + surviving upsert
- [ ] version=0 的 fan-out 不会推进 tombstone

### 权限与可用性
- [ ] `i32::MAX` 权限策略当前是否可接受
- [ ] `meili_ready` 切换时机足够保守
- [ ] Meili / embedding 宕机时退化不影响主业务

## 19. 关键文件索引

### Search 主域
- `src/domains/search.rs`
- `src/domains/search/api/handlers/search_handler.rs`
- `src/domains/search/api/routes.rs`
- `src/domains/search/dto/search_dto.rs`
- `src/domains/search/domain/model/search_document.rs`
- `src/domains/search/domain/repository/outbox_repo.rs`
- `src/domains/search/domain/repository/search_repo.rs`
- `src/domains/search/domain/repository/tombstone_repo.rs`
- `src/domains/search/domain/service/search_service.rs`
- `src/domains/search/infra/impl_service/search_service_impl.rs`
- `src/domains/search/infra/impl_service/filter_utils.rs`
- `src/domains/search/infra/impl_service/rrf.rs`
- `src/domains/search/infra/impl_service/sql_fallback.rs`
- `src/domains/search/infra/meilisearch/index_setup.rs`
- `src/domains/search/infra/meilisearch/meilisearch_client.rs`
- `src/domains/search/infra/meilisearch/meilisearch_repo.rs`
- `src/domains/search/infra/embedding/embedding_client.rs`
- `src/domains/search/infra/embedding/embedding_service.rs`
- `src/domains/search/infra/indexer/indexer_service.rs`
- `src/domains/search/infra/indexer/event_processor.rs`
- `src/domains/search/infra/indexer/full_sync.rs`
- `src/domains/search/infra/indexer/reconciliation.rs`
- `src/domains/search/infra/outbox_repo_impl.rs`
- `src/domains/search/infra/tombstone_repo_impl.rs`

### Luna 联动改造
- `src/domains/luna/infra/search_outbox.rs`
- `src/domains/luna/infra/impl_service/record.rs`
- `src/domains/luna/infra/impl_service/director.rs`
- `src/domains/luna/infra/impl_service/genre.rs`
- `src/domains/luna/infra/impl_service/idol.rs`
- `src/domains/luna/infra/impl_service/label.rs`
- `src/domains/luna/infra/impl_service/series.rs`
- `src/domains/luna/infra/impl_service/studio.rs`
- `src/domains/luna/infra/impl_repository/record_loader.rs`
- `src/domains/luna/infra/impl_repository/record.rs`
- `src/domains/luna/infra/impl_repository/entity_repo_macro.rs`

### 接线与表结构
- `src/app.rs`
- `src/main.rs`
- `src/common/app_state.rs`
- `src/common/bootstrap.rs`
- `src/common/config.rs`
- `migration/src/m20260408_000001_create_search_tables.rs`
- `src/entities/search_sync_events.rs`
- `src/entities/search_document_versions.rs`
