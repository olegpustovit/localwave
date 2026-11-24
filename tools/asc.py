#!/usr/bin/env python3
"""App Store Connect API helper for TestFlight releases of com.olegpustovit.localwave.

Reads ASC_KEY_ID, ASC_ISSUER_ID and the key (ASC_KEY_PATH or ~/.appstoreconnect/private_keys/AuthKey_<id>.p8).
Signs the ES256 token with openssl, so it needs no third-party modules. Never prints the key or token.

  asc.py versions                       (app ASC_APP_ID, default 6830623498, must have this bundle ID) latest App Store versions and TestFlight builds per platform
  asc.py certificates                   list signing certificates (type, name, expiry)
  asc.py create-certificate TYPE        DISTRIBUTION or MAC_INSTALLER_DISTRIBUTION: CSR from a new login-keychain key
  asc.py mac-profile                    create and download a MAC_APP_STORE profile for the bundle ID
  asc.py wait PLATFORM BUILD            wait until the build has processed (IOS or MAC_OS)
  asc.py testflight PLATFORM BUILD FILE set What to Test from FILE and add the build to the internal groups
  asc.py --self-test                    offline check of the signature conversion
"""
import base64
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

BUNDLE_ID = "com.olegpustovit.localwave"
API = "https://api.appstoreconnect.apple.com/v1/"


def b64(data):
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()


def der_to_raw(der):
    """An ECDSA DER signature (SEQUENCE of r, s) as the 64-byte r||s JWS form."""
    assert der[0] == 0x30, "not a DER sequence"
    at, parts = 2, []
    for _ in range(2):
        assert der[at] == 0x02, "not a DER integer"
        length = der[at + 1]
        value = der[at + 2:at + 2 + length].lstrip(b"\0")
        assert len(value) <= 32, "integer too long for P-256"
        parts.append(value.rjust(32, b"\0"))
        at += 2 + length
    return b"".join(parts)


def token():
    key_id, issuer = os.environ.get("ASC_KEY_ID"), os.environ.get("ASC_ISSUER_ID")
    if not key_id or not issuer:
        sys.exit("Set ASC_KEY_ID and ASC_ISSUER_ID.")
    key = Path(os.environ.get("ASC_KEY_PATH", Path.home() / ".appstoreconnect/private_keys" / f"AuthKey_{key_id}.p8"))
    header = b64(json.dumps({"alg": "ES256", "kid": key_id, "typ": "JWT"}).encode())
    now = int(time.time())
    claims = b64(json.dumps({"iss": issuer, "iat": now, "exp": now + 1100, "aud": "appstoreconnect-v1"}).encode())
    signing_input = f"{header}.{claims}".encode()
    der = subprocess.run(["openssl", "dgst", "-sha256", "-sign", str(key)], input=signing_input, capture_output=True, check=True).stdout
    return f"{header}.{claims}.{b64(der_to_raw(der))}"


def call(method, path, body=None, query=None):
    url = API + path + ("?" + urllib.parse.urlencode(query) if query else "")
    request = urllib.request.Request(url, method=method, data=json.dumps(body).encode() if body else None,
                                     headers={"Authorization": f"Bearer {token()}", "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            text = response.read()
            return json.loads(text) if text else {}
    except urllib.error.HTTPError as error:
        detail = json.loads(error.read() or b"{}").get("errors", [{}])[0]
        sys.exit(f"{method} {path}: HTTP {error.code} {detail.get('title', '')}: {detail.get('detail', '')}")


def app_id():
    """The owner's App Store Connect app (ASC_APP_ID, default 6830623498); refuses it unless its bundle ID is ours."""
    app = os.environ.get("ASC_APP_ID", "6830623498")
    bundle = call("GET", f"apps/{app}")["data"]["attributes"]["bundleId"]
    if bundle != BUNDLE_ID:
        sys.exit(f"App {app} has bundle ID {bundle}, not {BUNDLE_ID}; refusing to release into it.")
    return app


def builds(app, platform, version=None):
    query = {"filter[app]": app, "filter[preReleaseVersion.platform]": platform, "sort": "-uploadedDate",
             "limit": "20", "include": "preReleaseVersion"}
    if version:
        query["filter[version]"] = version
    result = call("GET", "builds", query=query)
    versions = {item["id"]: item["attributes"]["version"] for item in result.get("included", [])}
    return [(build, versions.get(build["relationships"]["preReleaseVersion"]["data"]["id"])) for build in result["data"]]


def versions():
    app = app_id()
    for store in call("GET", f"apps/{app}/appStoreVersions", query={"limit": "10"})["data"]:
        attributes = store["attributes"]
        print(f"store {attributes['platform']} {attributes['versionString']} {attributes['appStoreState']}")
    for platform in ("IOS", "MAC_OS"):
        for build, marketing in builds(app, platform)[:5]:
            attributes = build["attributes"]
            print(f"testflight {platform} {marketing} ({attributes['version']}) {attributes['processingState']} {attributes['uploadedDate']}")


def certificates():
    for certificate in call("GET", "certificates", query={"limit": "50"})["data"]:
        attributes = certificate["attributes"]
        print(f"{attributes['certificateType']} {attributes['name']} expires {attributes['expirationDate']}")


def create_certificate(kind):
    if kind not in ("DISTRIBUTION", "MAC_INSTALLER_DISTRIBUTION"):
        sys.exit("TYPE must be DISTRIBUTION or MAC_INSTALLER_DISTRIBUTION")
    with tempfile.TemporaryDirectory() as directory:
        key, csr, cer, p12 = (Path(directory) / name for name in ("key.pem", "request.csr", "cert.cer", "identity.p12"))
        subprocess.run(["openssl", "req", "-new", "-newkey", "rsa:2048", "-nodes", "-keyout", key, "-out", csr,
                        "-subj", f"/CN=LocalWave {kind}/C=US"], check=True, capture_output=True)
        body = {"data": {"type": "certificates", "attributes": {"certificateType": kind, "csrContent": csr.read_text()}}}
        created = call("POST", "certificates", body)["data"]
        cer.write_bytes(base64.b64decode(created["attributes"]["certificateContent"]))
        password = b64(os.urandom(18))
        pem = Path(directory) / "cert.pem"
        subprocess.run(["openssl", "x509", "-inform", "DER", "-in", cer, "-out", pem], check=True)
        # -legacy: the keychain's PKCS#12 importer rejects OpenSSL 3's default encryption
        subprocess.run(["openssl", "pkcs12", "-export", "-legacy", "-inkey", key, "-in", pem, "-out", p12,
                        "-passout", f"pass:{password}"], check=True, capture_output=True)
        subprocess.run(["security", "import", p12, "-k", str(Path.home() / "Library/Keychains/login.keychain-db"),
                        "-P", password, "-T", "/usr/bin/codesign", "-T", "/usr/bin/productbuild"], check=True, capture_output=True)
    print(f"created {kind} {created['attributes']['name']} expires {created['attributes']['expirationDate']}")


def mac_profile():
    bundle = call("GET", "bundleIds", query={"filter[identifier]": BUNDLE_ID, "filter[platform]": "MAC_OS"})["data"]
    bundle = bundle or call("GET", "bundleIds", query={"filter[identifier]": BUNDLE_ID})["data"]
    if not bundle:
        sys.exit(f"No bundle ID record for {BUNDLE_ID}")
    certs = [c["id"] for c in call("GET", "certificates", query={"filter[certificateType]": "DISTRIBUTION"})["data"]]
    if not certs:
        sys.exit("No DISTRIBUTION certificate; run create-certificate DISTRIBUTION first.")
    body = {"data": {"type": "profiles", "attributes": {"name": f"LocalWave Mac App Store {time.strftime('%Y%m%d%H%M')}", "profileType": "MAC_APP_STORE"},
                     "relationships": {"bundleId": {"data": {"type": "bundleIds", "id": bundle[0]["id"]}},
                                       "certificates": {"data": [{"type": "certificates", "id": c} for c in certs]}}}}
    created = call("POST", "profiles", body)["data"]
    path = Path("target/bundle/LocalWave_Mac_App_Store.provisionprofile").resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(base64.b64decode(created["attributes"]["profileContent"]))
    print(path)


def find_build(platform, number):
    for build, marketing in builds(app_id(), platform, number):
        return build, marketing
    return None, None


def wait(platform, number):
    deadline = time.time() + 3600
    while time.time() < deadline:
        build, marketing = find_build(platform, number)
        state = build["attributes"]["processingState"] if build else "NOT_FOUND"
        print(f"{platform} {marketing} ({number}): {state}", flush=True)
        if state in ("VALID", "FAILED", "INVALID"):
            return state == "VALID"
        time.sleep(60)
    sys.exit("Timed out waiting for processing")


def testflight(platform, number, notes_file):
    build, _ = find_build(platform, number)
    if not build:
        sys.exit("Build not found")
    notes = Path(notes_file).read_text().strip()
    existing = call("GET", f"builds/{build['id']}/betaBuildLocalizations")["data"]
    english = [item for item in existing if item["attributes"]["locale"].startswith("en")]
    if english:
        call("PATCH", f"betaBuildLocalizations/{english[0]['id']}",
             {"data": {"type": "betaBuildLocalizations", "id": english[0]["id"], "attributes": {"whatsNew": notes}}})
    else:
        call("POST", "betaBuildLocalizations", {"data": {"type": "betaBuildLocalizations", "attributes": {"locale": "en-US", "whatsNew": notes},
                                                         "relationships": {"build": {"data": {"type": "builds", "id": build["id"]}}}}})
    groups = [g for g in call("GET", "betaGroups", query={"filter[app]": app_id()})["data"] if g["attributes"]["isInternalGroup"]]
    for group in groups:
        call("POST", f"betaGroups/{group['id']}/relationships/builds", {"data": [{"type": "builds", "id": build["id"]}]})
        print(f"added {platform} ({number}) to internal group {group['attributes']['name']}")
    if not groups:
        print("no internal beta group found")


def self_test():
    r, s = bytes(range(1, 33)), b"\x80" + bytes(31)
    der = b"\x30\x45" + b"\x02\x20" + r + b"\x02\x21\x00" + s
    assert der_to_raw(der) == r + s
    short = b"\x30\x25" + b"\x02\x01\x07" + b"\x02\x20" + r
    assert der_to_raw(short) == bytes(31) + b"\x07" + r
    print("self-test ok")


def main():
    args = sys.argv[1:]
    commands = {"versions": versions, "certificates": certificates, "create-certificate": create_certificate,
                "mac-profile": mac_profile, "wait": wait, "testflight": testflight, "--self-test": self_test}
    if not args or args[0] not in commands:
        sys.exit(__doc__)
    commands[args[0]](*args[1:])


if __name__ == "__main__":
    main()
