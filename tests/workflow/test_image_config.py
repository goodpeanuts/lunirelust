"""Read-only validation of the real Compose merge; no Docker daemon is used."""
import json
import os
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ImageConfigurationTests(unittest.TestCase):
    def test_merged_image_stack_has_only_isolated_test_database_and_named_assets(self):
        image = 'sha256:' + 'a' * 64
        project = 'lunirelust-image-config-regression'
        environment = dict(os.environ, LUNIRELUST_CI_IMAGE_ID=image,
                           COMPOSE_PROFILES='hybrid', COMPOSE_FILE='/unrelated/compose.yml',
                           POSTGRES_USER='inherited-user', DATABASE_URL='inherited-database',
                           DOCKER_HOST='unix:///nonexistent-config-only-daemon')
        command = [str(ROOT / 'scripts/env-run.sh'), '.env.test', 'docker', 'compose',
                   '--project-directory', str(ROOT), '--project-name', project, '--env-file', '/dev/null',
                   '-f', 'compose.middleware.yml', '-f', 'compose.test.yml',
                   '-f', 'scripts/ci/compose.image.yml', 'config', '--format', 'json']
        result = subprocess.run(command, cwd=ROOT, env=environment, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        model = json.loads(result.stdout)
        self.assertEqual(set(model['services']), {'app', 'postgres'})
        for service in model['services'].values():
            self.assertFalse(service.get('ports'))
            for mount in service['volumes']:
                self.assertEqual(mount['type'], 'volume')
                volume = model['volumes'][mount['source']]
                self.assertEqual(volume['name'], project + '_' + mount['source'])
                self.assertFalse(volume.get('external', False))
        self.assertEqual(set(model['volumes']), {'image-assets', 'postgres-data'})
        app = model['services']['app']
        self.assertEqual(app['image'], image)
        self.assertEqual(app['pull_policy'], 'never')
        self.assertEqual(app['environment']['APP_ENV'], 'test')
        self.assertEqual(app['environment']['SEARCH_BACKEND'], 'postgres')
        self.assertEqual(app['environment']['DATABASE_URL'],
                         'postgres://luna_test:luna_test@postgres:5432/luna_test')
        self.assertEqual(app['volumes'][0]['target'], '/app/assets')
        postgres = model['services']['postgres']
        self.assertEqual(postgres['environment']['POSTGRES_USER'], 'luna_test')
        self.assertEqual(postgres['environment']['POSTGRES_DB'], 'luna_test')
        self.assertEqual(postgres['environment']['POSTGRES_PASSWORD'], 'luna_test')
        self.assertEqual(postgres['volumes'][0]['target'], '/var/lib/postgresql/data')


if __name__ == '__main__':
    unittest.main()
