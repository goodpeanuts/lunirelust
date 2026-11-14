#!/usr/bin/env python3
"""Hold the fixed test project's lock until the workflow and cleanup finish."""

import fcntl
import os
import signal
import stat
import subprocess
import sys


LOCK_PATH = "/tmp/lunirelust-test.lock"


def verify_descriptor(descriptor):
    opened = os.fstat(descriptor)
    named = os.lstat(LOCK_PATH)
    if (not stat.S_ISREG(opened.st_mode) or opened.st_nlink != 1
            or (opened.st_dev, opened.st_ino) != (named.st_dev, named.st_ino)):
        raise OSError("invalid lock file")
    fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)


def main():
    try:
        if sys.argv[1] == "--check":
            verify_descriptor(int(sys.argv[2]))
            return 0
        descriptor = os.open(LOCK_PATH, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
        verify_descriptor(descriptor)
    except (OSError, ValueError, IndexError):
        print("test infrastructure is busy or its shared lock is unavailable; "
              "wait for the other test workflow to finish", file=sys.stderr)
        return 75

    child = None
    received_signal = 0

    def forward(signum, _frame):
        nonlocal received_signal
        received_signal = signum
        if child is not None:
            try:
                os.killpg(child.pid, signum)
            except ProcessLookupError:
                pass

    signal.signal(signal.SIGINT, forward)
    signal.signal(signal.SIGTERM, forward)
    environment = dict(os.environ, LUNIRELUST_TEST_LOCK_FD=str(descriptor))
    # A separate process group lets Ctrl-C/TERM reach Cargo and its descendants.
    # Keep this supervisor alive (and the lock held) while Bash performs cleanup.
    child = subprocess.Popen(sys.argv[1:], env=environment, pass_fds=(descriptor,), start_new_session=True)
    if received_signal:
        forward(received_signal, None)
    status = child.wait()
    return 128 + received_signal if received_signal else (status if status >= 0 else 128 - status)


if __name__ == "__main__":
    sys.exit(main())
