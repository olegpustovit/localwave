#!/usr/bin/env python3
"""Fast, offline contracts for the full Rust app's iOS command."""
import contextlib
import hashlib
import io
import json
from pathlib import Path
import plistlib
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

import ios


def phone(udid="phone-1", transport="wired", modern=True, platform="iOS", kind="iPhone", pairing="paired", reality="physical"):
    hardware = dict(udid=udid, platform=platform, deviceType=kind, reality=reality)
    connection = dict(transportType=transport, pairingState=pairing)
    if modern:
        return dict(identifier="core-" + udid, properties=dict(hardware=hardware, connection=connection))
    return dict(identifier="core-" + udid, hardwareProperties=hardware, connectionProperties=connection)


def inventory(*devices):
    return dict(result=dict(devices=devices))


class IOSCommandTests(unittest.TestCase):
    def setUp(self):
        self.enterContext(patch.dict(ios.os.environ, {"IOS_BUNDLE_ID": ios.BUNDLE_ID, "IOS_TEAM_ID": "", "IOS_DEVICE": "", "IOS_SDK": "iphoneos"}))

    def test_usb_selection_handles_both_xcode_schemas_and_ignores_watch_wifi_and_simulators(self):
        for modern in (True, False):
            data = inventory(phone(modern=modern), phone("watch", platform="watchOS", kind="appleWatch"),
                             phone("wifi", transport="localNetwork"), phone("sim", transport="sameMachine", reality="simulated"))
            self.assertEqual(ios.choose_phone(data), "phone-1")
            self.assertEqual(ios.choose_phone(data, "wifi"), "wifi")
            self.assertEqual(ios.choose_phone(data, "core-phone-1"), "phone-1")

    def test_usb_selection_rejects_ambiguous_unpaired_and_invalid_identifiers(self):
        for data in (inventory(), inventory(phone(), phone("phone-2")), inventory(phone(pairing="unpaired")),
                     inventory(phone(udid="bad\n--device")), inventory(phone(platform="tvOS", kind="AppleTV"))):
            with self.assertRaises(ValueError):
                ios.choose_phone(data)
        self.assertEqual(ios.choose_phone(inventory(phone(), phone("phone-2")), "phone-2"), "phone-2")

    def test_simulator_selection_uses_available_ios_devices_and_explicit_shutdown_devices(self):
        device = dict(udid="sim-1", isAvailable=True, state="Booted", deviceTypeIdentifier="com.apple.CoreSimulator.SimDeviceType.iPhone-16e")
        watch = dict(device, udid="watch", deviceTypeIdentifier="com.apple.CoreSimulator.SimDeviceType.Apple-Watch")
        data = dict(devices={"iOS": [device], "watchOS": [watch]})
        self.assertEqual(ios.choose_simulator(data, "simulator"), device)
        device["state"] = "Shutdown"
        self.assertEqual(ios.choose_simulator(data, "sim-1"), device)
        with self.assertRaises(ValueError):
            ios.choose_simulator(data, "simulator")
        self.assertIsNone(ios.choose_simulator(data, "physical-device"))
        with self.assertRaises(ValueError):
            ios.choose_simulator(data, "watch")
        device["state"] = "Booted"
        data["devices"]["iOS"].append(dict(device, udid="sim-2"))
        with self.assertRaises(ValueError):
            ios.choose_simulator(data, "simulator")

    def test_teams_choose_a_unique_valid_team_or_the_project_team_without_guessing(self):
        self.assertEqual(ios.choose_team({"ABCDEFGHIJ"}, None, "0123456789"), "ABCDEFGHIJ")
        self.assertEqual(ios.choose_team(set(), None, "0123456789"), "0123456789")
        self.assertEqual(ios.choose_team({"ABCDEFGHIJ", "0123456789"}, None, "0123456789"), "0123456789")
        self.assertEqual(ios.choose_team(set(), "ABCDEFGHIJ", None), "ABCDEFGHIJ")
        for teams, requested in (({"ABCDEFGHIJ", "0123456789"}, None), (set(), "bad\nTEAMID"), (set(), None)):
            with self.assertRaises(ValueError):
                ios.choose_team(teams, requested, None)

    def test_certificate_team_comes_from_ou_of_a_valid_identity_not_the_common_name(self):
        certificate = b"-----BEGIN CERTIFICATE-----\nvalid\n-----END CERTIFICATE-----"
        old = b"-----BEGIN CERTIFICATE-----\nexpired\n-----END CERTIFICATE-----"
        digest = hashlib.sha1(b"valid-der").hexdigest().upper()

        def output(command, **kwargs):
            if command[0] == "security":
                return f'  1) {digest} "Apple Development: Person (NOTATEAMID)"\n'
            if "-outform" in command:
                return b"valid-der" if kwargs["input"] == certificate else b"expired-der"
            self.assertEqual(kwargs["input"], certificate, "expired/unavailable identities must be ignored")
            return b"subject=\n    CN=Apple Development: Person (NOTATEAMID)\n    OU = ABCDEFGHIJ\n"

        with patch.object(ios.subprocess, "check_output", side_effect=output), patch.object(ios.subprocess, "run", return_value=SimpleNamespace(returncode=0, stdout=certificate + old)):
            self.assertEqual(ios.development_teams(), {"ABCDEFGHIJ"})
        with patch.object(ios.subprocess, "check_output", return_value=""), patch.object(ios.subprocess, "run") as certificates:
            self.assertEqual(ios.development_teams(), set())
            certificates.assert_not_called()

    def test_build_commands_use_the_full_static_rust_app_and_xcode_automatic_signing(self):
        for sdk, team, profile in (("iphoneos", "ABCDEFGHIJ", "release"), ("iphoneos", None, "debug"), ("iphonesimulator", None, "debug")):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for name in ("pxui/Cargo.toml", "pxui/packaging/build-sdl-ios.sh", "pxui/packaging/sdl3.lock", f"target/sdl-{sdk}/lib/libSDL3.a"):
                    path = root / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.touch()
                with patch.object(ios, "ROOT", root), patch.object(ios.subprocess, "check_call") as call, patch.object(ios.subprocess, "check_output", return_value="\n".join(ios.TARGETS.values())), patch.object(ios, "verify_bundle"), contextlib.redirect_stdout(io.StringIO()):
                    bundle = ios.build(sdk, profile, ios.BUNDLE_ID, "phone-1" if team else None, team)
                cargo, xcode = [entry.args[0] for entry in call.call_args_list]
                self.assertIn("lw_bin", cargo)
                self.assertEqual(cargo[cargo.index("--bin") + 1], "localwave")
                self.assertEqual(cargo[cargo.index("--features") + 1], "pxui/static")
                self.assertEqual(cargo[cargo.index("--target") + 1], ios.TARGETS[sdk])
                self.assertEqual("--release" in cargo, profile == "release")
                self.assertEqual(call.call_args_list[0].kwargs["env"]["CARGO_TARGET_DIR"], str(root / "target"))
                self.assertEqual(xcode[xcode.index("-derivedDataPath") + 1], str(root / "target/ios"))
                if team:
                    self.assertIn("DEVELOPMENT_TEAM=ABCDEFGHIJ", xcode)
                    self.assertIn("-allowProvisioningUpdates", xcode)
                    self.assertIn("-allowProvisioningDeviceRegistration", xcode)
                    self.assertIn("id=phone-1", xcode)
                elif sdk == "iphoneos":
                    self.assertIn("CODE_SIGNING_ALLOWED=NO", xcode)
                    self.assertIn("-unsigned", str(bundle))
                else:
                    self.assertIn("CODE_SIGN_IDENTITY=-", xcode)
                self.assertEqual(bundle.name, "LocalWave.app")

    def test_bundle_verification_rejects_wrong_binary_platform_and_external_dylibs(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory) / "LocalWave.app"
            bundle.mkdir()
            with (bundle / "Info.plist").open("wb") as stream:
                plistlib.dump(dict(CFBundleIdentifier=ios.BUNDLE_ID, CFBundleExecutable="localwave", MinimumOSVersion="18.0", UIBackgroundModes=["audio"], CFBundleIcons={"CFBundlePrimaryIcon": {}}), stream)
            (bundle / "PrivacyInfo.xcprivacy").touch()
            for failure in (None, "uuid", "platform", "dylib", "profile"):
                def output(command, **_):
                    if "dwarfdump" in command:
                        uuid = "WRONG" if failure == "uuid" and str(bundle) in command[-1] else "SAME"
                        return f"UUID: {uuid} (arm64) {command[-1]}\n"
                    if "vtool" in command:
                        return "platform IOS\nminos 18.0\n" if failure in ("platform", "profile") else "platform IOSSIMULATOR\nminos 18.0\n"
                    return "localwave:\n\t" + ("@rpath/libpxui.dylib" if failure == "dylib" else "/usr/lib/libSystem.B.dylib") + " (compatibility version 1)\n"
                with patch.object(ios.subprocess, "check_output", side_effect=output), patch.object(ios.subprocess, "check_call") as call:
                    if failure:
                        messages = {"uuid": "built Rust", "platform": "wrong iOS", "dylib": "system dylibs", "profile": "provisioning profile"}
                        with self.assertRaisesRegex(ValueError, messages[failure]):
                            ios.verify_bundle(bundle, Path(directory) / "rust-binary", "iphoneos" if failure == "profile" else "iphonesimulator", ios.BUNDLE_ID, True)
                    else:
                        ios.verify_bundle(bundle, Path(directory) / "rust-binary", "iphonesimulator", ios.BUNDLE_ID, True)
                        self.assertIn("codesign", call.call_args.args[0])

    def test_shipping_bundle_id_and_invalid_ids_are_rejected_before_device_or_build_work(self):
        for identifier in ("com.olegpustovit.localwave", "bad\nPRODUCT_NAME=x", "com..example", "com.example."):
            with patch.object(ios.sys, "argv", ["ios.py", "run", "--bundle-id", identifier]), patch.object(ios, "resolve_device") as discover, patch.object(ios, "build") as build, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):
                    ios.main()
                discover.assert_not_called()
                build.assert_not_called()

    def test_invalid_sdk_default_fails_before_build(self):
        with patch.dict(ios.os.environ, {"IOS_SDK": "not-an-ios-sdk"}), patch.object(ios.sys, "argv", ["ios.py", "build"]), patch.object(ios, "build") as build, contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                ios.main()
            build.assert_not_called()

    def test_run_installs_before_launch_and_forwards_app_arguments_without_shell_expansion(self):
        for sdk in ios.TARGETS:
            with patch.object(ios.sys, "argv", ["ios.py", "run", "--", "--smoke-test", "literal$(value)"]), patch.object(ios.sys, "platform", "darwin"), patch.object(ios.os, "chdir"), patch.object(ios, "resolve_device", return_value=(sdk, "device-id")), patch.object(ios, "development_teams", return_value={"ABCDEFGHIJ"}), patch.object(ios, "build", return_value=Path("LocalWave.app")), patch.object(ios.subprocess, "check_call") as call, contextlib.redirect_stdout(io.StringIO()):
                ios.main()
            commands = [entry.args[0] for entry in call.call_args_list]
            self.assertIn("install", commands[0])
            self.assertIn("launch", commands[1])
            self.assertEqual(commands[1][-2:], ["--smoke-test", "literal$(value)"])
            self.assertIn(ios.BUNDLE_ID, commands[1])


if __name__ == "__main__":
    unittest.main()
