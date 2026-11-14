#!/usr/bin/env python3
"""Build the full Rust app; run signs, installs and launches it on a selected iOS device."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PROJECT = ROOT / "packaging/ios/LocalWave.xcodeproj"
BUNDLE_ID = "com.olegpustovit.localwave.rust"
TARGETS = {"iphoneos": "aarch64-apple-ios", "iphonesimulator": "aarch64-apple-ios-sim"}


def choose_phone(inventory, requested=None):
    phones = []
    for device in inventory["result"]["devices"]:
        properties = device.get("properties", {})
        hardware = properties.get("hardware", device.get("hardwareProperties", {}))
        connection = properties.get("connection", device.get("connectionProperties", {}))
        if (hardware.get("platform") != "iOS" or hardware.get("deviceType") not in ("iPhone", "iPad")
                or hardware.get("reality") == "simulated" or connection.get("pairingState") != "paired"):
            continue
        udid = hardware.get("udid")
        if not isinstance(udid, str) or not re.fullmatch(r"[A-Za-z0-9-]+", udid):
            continue
        if requested:
            if requested not in (udid, device.get("identifier")):
                continue
        elif connection.get("transportType") != "wired":
            continue
        phones.append(udid)
    if len(phones) != 1:
        if phones:
            raise ValueError("Multiple iOS devices: set IOS_DEVICE to one of " + ", ".join(phones))
        raise ValueError("No paired USB iPhone/iPad found. Connect and unlock it, trust this Mac and enable Developer Mode. "
                         "IOS_DEVICE=simulator explicitly selects a booted simulator.")
    return phones[0]


def choose_simulator(inventory, requested):
    matches = []
    for group in inventory["devices"].values():
        for device in group:
            phone = device.get("deviceTypeIdentifier", "").split(".")[-1].startswith(("iPhone", "iPad"))
            selected = device.get("state") == "Booted" and phone if requested == "simulator" else device.get("udid") == requested
            if device.get("isAvailable") and selected:
                matches.append(device)
    if not matches:
        if requested == "simulator":
            raise ValueError("Boot an iOS simulator in Xcode, or set IOS_DEVICE to its UDID.")
        return None
    if len(matches) != 1:
        raise ValueError("Multiple booted simulators: set IOS_DEVICE to the desired simulator UDID.")
    device = matches[0]
    # simctl also lists watchOS and other runtimes; this app only targets iOS.
    if not device.get("deviceTypeIdentifier", "").split(".")[-1].startswith(("iPhone", "iPad")):
        raise ValueError("Select an iPhone or iPad simulator.")
    return device


def development_teams():
    identities = subprocess.check_output(["security", "find-identity", "-v", "-p", "codesigning"], text=True)
    valid = set(re.findall(r'^\s*\d+\) ([0-9A-F]{40}) "(?:Apple Development|iPhone Developer):', identities, re.M))
    if not valid:
        return set()
    teams = set()
    for kind in ("Apple Development", "iPhone Developer"):
        result = subprocess.run(["security", "find-certificate", "-a", "-c", kind, "-p"], capture_output=True)
        if result.returncode not in (0, 44):
            raise ValueError("Could not read development certificates from the login keychain.")
        for certificate in re.findall(rb"-----BEGIN CERTIFICATE-----.*?-----END CERTIFICATE-----", result.stdout, re.S):
            der = subprocess.check_output(["openssl", "x509", "-outform", "DER"], input=certificate)
            if hashlib.sha1(der).hexdigest().upper() not in valid:
                continue
            subject = subprocess.check_output(["openssl", "x509", "-noout", "-subject", "-nameopt", "sep_multiline"], input=certificate).decode()
            teams.update(re.findall(r"^\s*OU\s*=\s*([A-Z0-9]{10})\s*$", subject, re.M))
    return teams


def choose_team(teams, requested, configured):
    if requested:
        if not re.fullmatch(r"[A-Z0-9]{10}", requested):
            raise ValueError("IOS_TEAM_ID must be a ten-character Apple development team ID.")
        return requested
    if len(teams) == 1:
        return next(iter(teams))
    if configured and (not teams or configured in teams):
        # Xcode can create a missing development certificate for the project's existing team.
        return configured
    if not teams:
        raise ValueError("No development team found. Add your Apple account in Xcode and set IOS_TEAM_ID.")
    raise ValueError("Multiple development teams: set IOS_TEAM_ID to " + ", ".join(sorted(teams)))


def resolve_device(requested):
    if requested:
        simulators = json.loads(subprocess.check_output(["xcrun", "simctl", "list", "devices", "--json"], text=True))
        simulator = choose_simulator(simulators, requested)
        if simulator:
            if simulator["state"] != "Booted":
                subprocess.check_call(["xcrun", "simctl", "boot", simulator["udid"]])
            subprocess.check_call(["xcrun", "simctl", "bootstatus", simulator["udid"], "-b"])
            return "iphonesimulator", simulator["udid"]
    with tempfile.TemporaryDirectory(prefix="localwave-ios-device-") as directory:
        output = Path(directory) / "devices.json"
        subprocess.check_call(["xcrun", "devicectl", "list", "devices", "--timeout", "30", "--json-output", str(output)], stdout=subprocess.DEVNULL)
        return "iphoneos", choose_phone(json.loads(output.read_text()), requested)


def build(sdk, profile, bundle_id, device=None, team=None):
    if not (ROOT / "pxui/Cargo.toml").is_file():
        subprocess.check_call(["git", "submodule", "update", "--init", "pxui"])
    target = TARGETS[sdk]
    installed = subprocess.check_output(["rustup", "target", "list", "--installed"], text=True).splitlines()
    if target not in installed:
        subprocess.check_call(["rustup", "target", "add", target])
    sdl = ROOT / "target" / f"sdl-{sdk}"
    archive = sdl / "lib/libSDL3.a"
    inputs = [ROOT / "pxui/packaging/build-sdl-ios.sh", ROOT / "pxui/packaging/sdl3.lock"]
    if not archive.is_file() or any(path.stat().st_mtime > archive.stat().st_mtime for path in inputs):
        subprocess.check_call([str(inputs[0]), sdk, str(ROOT / "target")])
    print(f"Building full Rust LocalWave ({profile}, {sdk})", flush=True)
    subprocess.check_call(["cargo", "build", "--locked", "-p", "lw_bin", "--bin", "localwave", "--features", "pxui/static",
                           "--target", target] + (["--release"] if profile == "release" else []),
                          env=dict(os.environ, SDL3_DIR=str(sdl), IPHONEOS_DEPLOYMENT_TARGET="18.0", CARGO_TARGET_DIR=str(ROOT / "target")))
    binary = ROOT / "target" / target / profile / "localwave"
    configuration = "Release" if profile == "release" else "Debug"
    command = ["xcodebuild", "-project", str(PROJECT), "-scheme", "LocalWave", "-configuration", configuration,
               "-sdk", sdk, "-destination", f"id={device}" if device else f"generic/platform={'iOS' if sdk == 'iphoneos' else 'iOS Simulator'}",
               "-destination-timeout", "30", "-derivedDataPath", str(ROOT / "target/ios"),
               f"LOCALWAVE_RUST_EXECUTABLE={binary}", f"PRODUCT_BUNDLE_IDENTIFIER={bundle_id}"]
    if team:
        command += [f"DEVELOPMENT_TEAM={team}", "CODE_SIGN_STYLE=Automatic", "CODE_SIGN_IDENTITY=Apple Development",
                    "-allowProvisioningUpdates", "-allowProvisioningDeviceRegistration"]
    else:
        command += ["CODE_SIGN_IDENTITY=-"] if sdk == "iphonesimulator" else ["CODE_SIGNING_ALLOWED=NO"]
    products = ROOT / "target/ios/Build/Products" / f"{configuration}-{sdk}"
    if sdk == "iphoneos" and not team:
        # An unsigned build must not inherit a previous run's signature or provisioning profile.
        products = products.with_name(products.name + "-unsigned")
        command += [f"CONFIGURATION_BUILD_DIR={products}"]
    subprocess.check_call(command + ["build"])
    bundle = products / "LocalWave.app"
    verify_bundle(bundle, binary, sdk, bundle_id, signed=sdk == "iphonesimulator" or team is not None)
    print(f"{'Signed' if sdk == 'iphonesimulator' or team else 'Unsigned'} full Rust app: {bundle}", flush=True)
    return bundle


def verify_bundle(bundle, binary, sdk, bundle_id, signed):
    with (bundle / "Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    if info["CFBundleIdentifier"] != bundle_id or info["MinimumOSVersion"] != "18.0" or "audio" not in info.get("UIBackgroundModes", []):
        raise ValueError("Packaged app identity, deployment floor or audio background mode is incorrect.")
    executable = bundle / info["CFBundleExecutable"]
    expected_uuid = subprocess.check_output(["xcrun", "dwarfdump", "--uuid", str(binary)], text=True).splitlines()
    actual_uuid = subprocess.check_output(["xcrun", "dwarfdump", "--uuid", str(executable)], text=True).splitlines()
    if [line.split()[:3] for line in expected_uuid] != [line.split()[:3] for line in actual_uuid] or not actual_uuid:
        raise ValueError("The bundle does not contain the built Rust LocalWave executable.")
    subprocess.check_call(["xcrun", "lipo", "-verify_arch", "arm64", str(executable)])
    platform = "IOS" if sdk == "iphoneos" else "IOSSIMULATOR"
    metadata = subprocess.check_output(["xcrun", "vtool", "-show-build", str(executable)], text=True)
    if not re.search(rf"^\s*platform {platform}$", metadata, re.M) or not re.search(r"^\s*minos 18\.0$", metadata, re.M):
        raise ValueError("The Rust executable has the wrong iOS platform or deployment floor.")
    dependencies = subprocess.check_output(["xcrun", "otool", "-L", str(executable)], text=True).splitlines()[1:]
    if any(not line.strip().startswith(("/System/Library/", "/usr/lib/")) for line in dependencies):
        raise ValueError("The iOS app must statically include pxui/SDL and only link system dylibs.")
    if not (bundle / "PrivacyInfo.xcprivacy").is_file() or not info.get("CFBundleIcons"):
        raise ValueError("The app is missing its privacy manifest or app icon.")
    if signed:
        subprocess.check_call(["codesign", "--verify", "--deep", "--strict", str(bundle)])
    if signed and sdk == "iphoneos" and not (bundle / "embedded.mobileprovision").is_file():
        raise ValueError("The device app has no embedded provisioning profile.")


def main():
    before, separator, application_args = sys.argv[1:], False, []
    if "--" in before:
        split = before.index("--")
        before, application_args, separator = before[:split], before[split + 1:], True
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("build", "run"))
    parser.add_argument("--profile", choices=("debug", "release"), default="debug")
    parser.add_argument("--sdk", choices=TARGETS, help="SDK for build; run chooses the destination SDK", default=os.environ.get("IOS_SDK", "iphoneos"))
    parser.add_argument("--device", default=os.environ.get("IOS_DEVICE"))
    parser.add_argument("--team", help="Development team for a physical device run", default=os.environ.get("IOS_TEAM_ID"))
    parser.add_argument("--bundle-id", default=os.environ.get("IOS_BUNDLE_ID", BUNDLE_ID))
    args = parser.parse_args(before)
    if args.sdk not in TARGETS:
        parser.error("IOS_SDK must be iphoneos or iphonesimulator.")
    if not re.fullmatch(r"[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+", args.bundle_id) or args.bundle_id == "com.olegpustovit.localwave":
        parser.error("Use a separate development bundle ID; the shipping Swift app must remain installed.")
    if separator and args.action != "run":
        parser.error("Application arguments after -- require run.")
    if sys.platform != "darwin":
        parser.error("iOS builds require a Mac with full Xcode selected by xcode-select.")
    os.chdir(ROOT)
    device = team = None
    sdk = args.sdk
    if args.action == "run":
        sdk, device = resolve_device(args.device)
        if sdk == "iphoneos":
            configured = re.search(r"^DEVELOPMENT_TEAM\s*=\s*([A-Z0-9]{10})\s*$", (ROOT / "packaging/ios/LocalWave.xcconfig").read_text(), re.M)
            team = choose_team(development_teams(), args.team, configured[1] if configured else None)
            print(f"Signing for team {team}; destination {device}", flush=True)
    bundle = build(sdk, args.profile, args.bundle_id, device, team)
    if args.action == "run":
        if sdk == "iphonesimulator":
            subprocess.check_call(["xcrun", "simctl", "install", device, str(bundle)])
            subprocess.check_call(["xcrun", "simctl", "launch", "--terminate-running-process", device, args.bundle_id] + application_args)
        else:
            subprocess.check_call(["xcrun", "devicectl", "device", "install", "app", "--device", device, "--timeout", "60", str(bundle)])
            subprocess.check_call(["xcrun", "devicectl", "device", "process", "launch", "--device", device, "--timeout", "30", "--terminate-existing", args.bundle_id, "--"] + application_args)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.SubprocessError) as error:
        print(f"LocalWave iOS: {error}", file=sys.stderr)
        sys.exit(1)
