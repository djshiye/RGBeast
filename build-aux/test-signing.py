#!/usr/bin/python3
"""Prove that dnf-repo.py publishes only packages signed with the repository key.

    test-signing.py <unsigned.rpm>

Makes two throwaway keys, the repository key and another one, and tells the system rpm database
to trust the other key, the way a Fedora system trusts Fedora's keys. A package signed with the
other key must still be refused, and so must an unsigned one. Needs gpg, rpm-sign and python3-rpm.
"""
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile

here = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("dnf_repo", os.path.join(here, "dnf-repo.py"))
dnf_repo = importlib.util.module_from_spec(spec)
spec.loader.exec_module(dnf_repo)


def run(*cmd, **kw):
    return subprocess.run(cmd, check=True, text=True, **kw)


def make_key(name, out):
    run("gpg", "--batch", "--quiet", "--passphrase", "", "--quick-gen-key", name, "rsa2048",
        "sign", "1d")
    fpr = secret_fpr(name)
    with open(out, "w") as f:
        run("gpg", "--batch", "--armor", "--export-options", "export-minimal", "--export", fpr,
            stdout=f)
    return fpr


def secret_fpr(name):
    out = run("gpg", "--batch", "--with-colons", "--list-secret-keys", name,
              capture_output=True).stdout
    return next(l.split(":")[9] for l in out.splitlines() if l.startswith("fpr:"))


def sign(path, fpr):
    run("rpmsign", "--define", f"_gpg_name {fpr}", "--addsign", path, stdout=subprocess.DEVNULL)


failures = []


def expect(what, ok):
    print(f"{'PASS' if ok else 'FAIL'}  {what}", flush=True)
    if not ok:
        failures.append(what)


def main():
    src = sys.argv[1]
    work = tempfile.mkdtemp(prefix="test-signing-")
    os.environ["GNUPGHOME"] = os.path.join(work, "gnupg")
    os.mkdir(os.environ["GNUPGHOME"], 0o700)

    repo_key, other_key = os.path.join(work, "repo.asc"), os.path.join(work, "other.asc")
    repo_fpr = make_key("Repository test key", repo_key)
    other_fpr = make_key("Other test key", other_key)

    pkgs = {}
    for name, fpr in (("repo", repo_fpr), ("other", other_fpr), ("unsigned", None)):
        pkgs[name] = os.path.join(work, f"{name}.rpm")
        shutil.copy2(src, pkgs[name])
        if fpr:
            sign(pkgs[name], fpr)

    # The system trusts the other key: the old check (rpm -K against the system database)
    # accepted a package signed with it.
    run("rpmkeys", "--import", other_key)
    system = subprocess.run(["rpmkeys", "--checksig", pkgs["other"]], capture_output=True, text=True)
    print(f"      system rpm database says: {system.stdout.strip()}")

    trusted = dnf_repo.keyring(repo_key)
    expect("package signed with the repository key is accepted",
           dnf_repo.signed_ok(pkgs["repo"], trusted))
    expect("package signed with another trusted key is refused",
           not dnf_repo.signed_ok(pkgs["other"], trusted))
    expect("unsigned package is refused", not dnf_repo.signed_ok(pkgs["unsigned"], trusted))

    expect("the repository key passes the fingerprint pin",
           dnf_repo.check_key(repo_key, repo_fpr) == repo_fpr)
    for what, keyfile, pin in (
        ("a different key fails the fingerprint pin", other_key, repo_fpr),
        ("a key file holding two keys is refused", None, None),
    ):
        if keyfile is None:
            keyfile = os.path.join(work, "both.asc")
            with open(keyfile, "w") as f:
                f.write(open(repo_key).read() + open(other_key).read())
        try:
            dnf_repo.check_key(keyfile, pin)
            expect(what, False)
        except SystemExit:
            expect(what, True)

    run("gpgconf", "--kill", "all")
    shutil.rmtree(work, ignore_errors=True)
    if failures:
        sys.exit(f"{len(failures)} signing check(s) failed")
    print("all signing checks passed")


if __name__ == "__main__":
    main()
