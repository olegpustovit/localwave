#!/usr/bin/env python3
"""Collect one bounded xctrace window and compare it with the machine's baseline."""
import ctypes
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET


class Usage(ctypes.Structure):
    # rusage_info_v2 from sys/resource.h. CPU times use Mach absolute ticks.
    _fields_ = [("uuid", ctypes.c_uint8 * 16)] + [
        (name, ctypes.c_uint64)
        for name in (
            "user", "system", "idle_wakeups", "interrupt_wakeups", "pageins",
            "wired", "resident", "footprint", "start", "exit", "child_user",
            "child_system", "child_idle_wakeups", "child_interrupt_wakeups",
            "child_pageins", "child_elapsed", "bytes_read", "bytes_written",
        )
    ]


def usage(pid):
    sample = Usage()
    libproc = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
    if libproc.proc_pid_rusage(pid, 2, ctypes.byref(sample)):
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error))
    return sample


def cpu_timebase():
    scale = (ctypes.c_uint32 * 2)()
    if ctypes.CDLL(None).mach_timebase_info(ctypes.byref(scale)) or not scale[1]:
        raise RuntimeError("Could not read the Mach timebase")
    return scale[0] / scale[1]


def table(trace, schema):
    out = subprocess.check_output([
        "xcrun", "xctrace", "export", "--input", str(trace),
        "--xpath", f'/trace-toc/run/data/table[@schema="{schema}"]',
    ])
    tree = ET.fromstring(out)
    if tree.find(".//node") is None:
        raise RuntimeError(f"xctrace trace has no {schema} table")
    references = {element.get("id"): element for element in tree.iter() if element.get("id")}
    columns = [column.text for column in tree.findall(".//schema/col/mnemonic")]
    return [
        dict(zip(columns, (references.get(element.get("ref"), element) for element in row), strict=True))
        for row in tree.findall(".//row")
    ]


def trace_metrics(trace, pid, seconds):
    def belongs(row):
        return row["process"].get("fmt", "").endswith(f" ({pid})")

    presents = sum(belongs(row) for row in table(trace, "ca-client-present-request"))
    gpu_ms = {}
    for row in table(trace, "metal-gpu-intervals"):
        if not belongs(row) or row["duration"].text is None:
            continue
        label = row["event-label"].get("fmt", "")
        match = re.search(r"(Render Command \d+|Compute Command \d+|Blit Command \d+)", label)
        if match:
            channel = row["channel-name"].get("fmt", "unknown")
            key = f"{match[1]} / {channel}"
            gpu_ms[key] = gpu_ms.get(key, 0) + int(row["duration"].text) / (1_000_000 * seconds)
    return presents, gpu_ms


def counters(line):
    return {key: int(value) for key, value in re.findall(r"(\w+)=(\d+)\b", line)}


def compare(result, expected):
    if result["seconds"] != expected["seconds"]:
        raise ValueError(f'Baseline uses {expected["seconds"]} seconds')
    if result["drawable"] != expected["drawable"]:
        raise ValueError(f'Baseline uses drawable {expected["drawable"]}, got {result["drawable"]}')
    if result["presents"] != expected["presents"]:
        raise ValueError(f'Presents changed from {expected["presents"]} to {result["presents"]}')
    actual = result["gpu_ms_per_s"]
    baseline = expected["gpu_ms_per_s"]
    for encoder in actual.keys() | baseline.keys():
        now, old = actual.get(encoder, 0), baseline.get(encoder, 0)
        if abs(now - old) > max(0.2, 0.10 * old):
            raise ValueError(f"{encoder} GPU ms/s changed from {old:.3f} to {now:.3f}")


def main(app, scenario, seconds):
    run_dir = app.parent
    log = run_dir / "app.log"
    trace = run_dir / "scenario.trace"
    env = dict(os.environ, PXUI_FRAME_REASONS="1", LOCALWAVE_ENERGY_SECONDS=str(seconds))
    with log.open("w") as output, subprocess.Popen(
        [app / "Contents/MacOS/localwave", f"--energy-{scenario}"],
        stdout=output, stderr=subprocess.STDOUT, env=env,
    ) as process:
        try:
            for _ in range(200):
                if "LOCALWAVE_ENERGY_READY " in log.read_text():
                    break
                if process.poll() is not None:
                    raise RuntimeError(f"Energy app exited during startup. See {log}")
                time.sleep(0.1)
            else:
                raise RuntimeError(f"Energy app did not become ready. See {log}")

            before = usage(process.pid)
            start = time.monotonic()
            power = None
            if os.environ.get("LOCALWAVE_ENERGY_POWER") == "1":
                power = subprocess.Popen(
                    ["sudo", "-n", "powermetrics", "--samplers", "gpu_power", "-i", "1000", "-n", str(seconds)],
                    stdout=subprocess.PIPE, text=True,
                )
            try:
                with (run_dir / "xctrace.log").open("w") as trace_log:
                    subprocess.run([
                        "xcrun", "xctrace", "record", "--template", "Metal System Trace",
                        "--attach", str(process.pid), "--no-prompt", "--time-limit", f"{seconds}s", "--output", str(trace),
                    ], stdout=trace_log, stderr=subprocess.STDOUT, check=True, timeout=seconds + 30)
                after = usage(process.pid)
                elapsed = time.monotonic() - start
            finally:
                if power:
                    power_output, _ = power.communicate(timeout=seconds + 5)
            process.wait(timeout=15)
            if process.returncode:
                raise RuntimeError(f"Energy app exited with {process.returncode}. See {log}")
        finally:
            if process.poll() is None:
                process.terminate()

    presents, gpu_ms = trace_metrics(trace, process.pid, seconds)
    lines = log.read_text().splitlines()
    ready, done = (
        counters(next(line for line in lines if line.startswith(f"LOCALWAVE_ENERGY_{mark} ")))
        for mark in ("READY", "DONE")
    )
    if scenario in {"playing-list", "pointer"} and not (presents and gpu_ms):
        raise RuntimeError(f"Visible playback trace has no presents or GPU encoder times. See {run_dir}")
    if done["draws"] < ready["draws"] or done["stepped"] < ready["stepped"]:
        raise RuntimeError(f"Energy frame-reason counters went backwards. See {log}")
    if env.get("LOCALWAVE_ENERGY_FIXED") == "1" and (presents != 160 or done["draws"] - ready["draws"] != 160):
        raise RuntimeError(f"Fixed replay requires 160 app draws and traced presents. See {run_dir}")
    drawable = [ready.pop("width"), ready.pop("height")]
    result = {
        "machine": subprocess.check_output(["sysctl", "-n", "hw.model"], text=True).strip(),
        "scenario": scenario,
        "seconds": seconds,
        "drawable": drawable,
        "presents": presents,
        "presents_per_s": presents / seconds,
        "gpu_ms_per_s": {key: round(value, 4) for key, value in sorted(gpu_ms.items())},
        "cpu_percent": round((after.user - before.user + after.system - before.system) * cpu_timebase() / elapsed / 1e7, 4),
        "idle_wakeups_per_s": round((after.idle_wakeups - before.idle_wakeups) / elapsed, 4),
        "app_counters": {key: done[key] - ready[key] for key in ready},
    }
    if power and power.returncode == 0:
        points = [float(p) for p in re.findall(r"GPU Power: ([\d.]+) mW", power_output)]
        if points:
            result["gpu_power_mw"] = sum(points) / len(points)
    output = Path("target/energy") / f"{scenario}.json"
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(output)
    if os.environ.get("LOCALWAVE_ENERGY_RECORD") != "1":
        baseline = json.loads(Path("docs/energy-baseline.json").read_text())
        if result["machine"] != baseline["model"]:
            raise ValueError(f'Baseline belongs to {baseline["label"]} ({baseline["model"]})')
        compare(result, baseline["scenarios"][scenario])


if __name__ == "__main__":
    main(Path(sys.argv[1]), sys.argv[2], int(sys.argv[3]))
