#!/usr/bin/env python3
"""Validate local workflow boundaries without connecting to any service."""

import json
import math
import os
from pathlib import Path
import sys
from urllib.parse import unquote, urlsplit


class UnsafeConfiguration(Exception):
    """A guard failure with a message that never includes configuration values."""


def require(condition, message):
    if not condition:
        raise UnsafeConfiguration(message)


def environment(mode, root):
    require(os.environ.get("APP_ENV") == mode, "APP_ENV does not match the workflow")
    identity = "luna_" + mode
    port = 5434 if mode == "dev" else 5433
    database_url = os.environ.get("DATABASE_URL", "")
    require(not any(ord(char) < 32 for char in database_url), "invalid DATABASE_URL")
    target = urlsplit(database_url)
    require(
        target.scheme in ("postgres", "postgresql")
        and target.hostname in ("127.0.0.1", "localhost")
        and target.port == port
        and target.username == identity
        and target.path == "/" + identity
        and not target.query
        and not target.fragment,
        "DATABASE_URL must use the fixed local " + mode + " database identity",
    )
    for name in ("POSTGRES_USER", "POSTGRES_DB"):
        require(os.environ.get(name) == identity, name + " does not match the workflow")
    password = os.environ.get("POSTGRES_PASSWORD", "")
    require(password and unquote(target.password or "") == password, "database passwords must match")
    if mode == "test":
        require(password == "luna_test", "test database credentials are fixed")
        require(os.environ.get("MEILI_MASTER_KEY") == "luna_test_master_key",
                "test MeiliSearch credentials are fixed")
    require(os.environ.get("MEILI_MASTER_KEY"), "MEILI_MASTER_KEY is required")
    require(os.environ.get("SEARCH_BACKEND", "postgres")
            in ("postgres", "meili-keyword", "meili-hybrid"), "invalid SEARCH_BACKEND")
    meili = urlsplit(os.environ.get("MEILI_URL", ""))
    require(
        meili.scheme == "http" and meili.hostname in ("127.0.0.1", "localhost")
        and meili.port == (7700 if mode == "dev" else 7701)
        and not meili.username and not meili.password and not meili.query
        and not meili.fragment and meili.path in ("", "/"),
        "MEILI_URL must use the fixed local " + mode + " endpoint",
    )
    data_root = root / ".local" / mode
    require(data_root.resolve() == data_root,
            "unsafe " + mode + " data root (symlink or path escape)")
    paths = {
        "ASSETS_HOME_PATH": "assets",
        "ASSETS_PUBLIC_PATH": "assets/public",
        "ASSETS_PRIVATE_PATH": "assets/private",
    }
    if mode == "dev":
        paths["ASSETS_DATA_PATH"] = "assets"
        for directory in ("postgres", "meilisearch"):
            require((data_root / directory).resolve() == data_root / directory,
                    "unsafe development middleware storage")
    for key, relative in paths.items():
        value = os.environ.get(key, "")
        require(".." not in Path(value).parts,
                "unsafe " + key + " ('..' path components are not allowed)")
        resolved = (root / value).resolve()
        if mode == "test":
            require(value and resolved == data_root / relative,
                    "unsafe " + key + " (must stay in local test assets)")
        else:
            # Preserve existing native asset layouts inside this checkout.
            require(value and root in resolved.parents,
                    "unsafe " + key + " (must stay inside the checkout)")
            protected = (".git", ".agents", ".codex", ".aws", "subm", ".local/test",
                         ".local/dev/postgres", ".local/dev/meilisearch")
            require(all(resolved != root / path and root / path not in resolved.parents
                        and resolved not in (root / path).parents for path in protected),
                    "unsafe " + key + " (must not overlap metadata or middleware/test data)")


def compose(mode, root, profile):
    model = json.load(sys.stdin)
    project = "lunirelust-" + mode
    require(model.get("name") == project, "unexpected Compose project")
    services = model["services"]
    expected = {"postgres"}
    if profile != "default":
        expected.add("meilisearch")
    if profile == "hybrid":
        expected.add("vllm")
    require(set(services) == expected, "unexpected active services for " + profile)
    for name in ("postgres", "meilisearch"):
        if name not in services:
            continue
        service = services[name]
        postgres = name == "postgres"
        require(int(service["mem_limit"]) > 0, "middleware memory limit must be positive")
        cpus = float(service["cpus"])
        require(math.isfinite(cpus) and cpus > 0, "middleware CPU limit must be positive")
        require(service.get("healthcheck", {}).get("test")
                and not service["healthcheck"].get("disable"), "missing middleware health check")
        reservations = service.get("deploy", {}).get("resources", {}).get("reservations", {})
        require(not service.get("devices") and not service.get("gpus")
                and not reservations.get("devices") and service.get("runtime") != "nvidia",
                "middleware must not request GPU devices")
        ports = service.get("ports", [])
        port = (5434 if mode == "dev" else 5433) if postgres else (
            7700 if mode == "dev" else 7701)
        require(len(ports) == 1 and ports[0].get("host_ip") == "127.0.0.1"
                and str(ports[0]["published"]) == str(port)
                and ports[0]["target"] == (5432 if postgres else 7700),
                "unsafe middleware port mapping")
        mounts = service.get("volumes", [])
        require(len(mounts) == 1 and mounts[0]["target"]
                == ("/var/lib/postgresql/data" if postgres else "/meili_data"),
                "unexpected middleware mount")
        mount = mounts[0]
        if mode == "dev":
            require(mount["type"] == "bind"
                    and Path(mount["source"]).resolve() == root / ".local/dev" / name,
                    "unsafe development middleware mount")
        else:
            require(mount["type"] == "volume" and mount["source"] == name + "-data",
                    "test middleware requires disposable named volumes")
            volume = model["volumes"][mount["source"]]
            require(volume.get("name") == project + "_" + mount["source"]
                    and not volume.get("external") and not volume.get("driver_opts")
                    and volume.get("driver", "local") == "local", "unsafe test volume definition")
        # `compose config` escapes literal dollars so its output can be reused.
        values = {
            key: value.replace("$$", "$") if isinstance(value, str) else value
            for key, value in service["environment"].items()
        }
        if postgres:
            require(values.get("POSTGRES_USER") == "luna_" + mode
                    and values.get("POSTGRES_DB") == "luna_" + mode
                    and values.get("POSTGRES_PASSWORD") == os.environ["POSTGRES_PASSWORD"],
                    "unexpected Compose database identity")
        else:
            require(values.get("MEILI_MASTER_KEY") == os.environ["MEILI_MASTER_KEY"],
                    "unexpected Compose MeiliSearch credentials")


try:
    operation, mode, directory = sys.argv[1:4]
    require(mode in ("dev", "test"), "expected dev or test workflow")
    root = Path(directory).resolve()
    if operation == "environment":
        environment(mode, root)
    elif operation == "compose":
        compose(mode, root, sys.argv[4])
    else:
        raise UnsafeConfiguration("unknown configuration check")
except UnsafeConfiguration as error:
    print("configuration check failed: " + str(error), file=sys.stderr)
    sys.exit(1)
except (ValueError, KeyError, IndexError, TypeError, OSError, RuntimeError):
    # Parsing errors can include raw values; report only our known guard messages.
    print("configuration check failed: invalid configuration format; "
          "check database identity, assets, and Compose settings", file=sys.stderr)
    sys.exit(1)
