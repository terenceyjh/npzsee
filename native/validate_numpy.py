# SPDX-License-Identifier: GPL-3.0-or-later
"""Independent NumPy oracle for the native parser and the supplied recording.

Run after cargo build --release. Outputs (including generated fixtures) stay in
validation-output, never in Git. Does not print experimental metadata values.
"""
from pathlib import Path
import argparse
import json
import os
import subprocess
import time
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "validation-output"
BENCH = Path(os.environ.get("NPZSEE_BENCH", ROOT / ("target/release/npzsee-bench.exe" if os.name == "nt" else "target/release/npzsee-bench")))


def probe(path, key, inspect=True):
    args = [str(BENCH), str(path), key]
    if inspect:
        args.append("--inspect")
    return json.loads(subprocess.check_output(args, text=True, encoding="utf-8"))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--recording", type=Path)
    args = parser.parse_args()
    OUT.mkdir(exist_ok=True)
    fixtures = {
        "long_text": np.array(["Full cell content — 中文 — русский — العربية\n" * 400]),
        "metadata_json": np.array(json.dumps({"setup": {"仪器": "测试🙂", "channels": [1, 2]}, "notes": "多行\n元数据"}, ensure_ascii=False)),
        "voltage": np.sin(np.arange(100_000) * .02),
        "cube": np.arange(2 * 3 * 4 * 5, dtype=np.float64).reshape(2, 3, 4, 5),
        "fortran": np.asfortranarray(np.arange(24, dtype=np.int32).reshape(2, 3, 4)),
        "unicode": np.array(["中文", "µV/√Hz", "🙂", "two\nlines", "embedded\0nul"]),
        "big_unicode": np.array(["中文", "🙂"]).astype(">U3"),
        "byte_strings": np.array([b"hello", b"world"], dtype="S8"),
        "signed_be": np.array([-32768, -1, 0, 32767], dtype=">i2"),
        "unsigned": np.array([0, 2**63 + 1, 2**64 - 1], dtype="u8"),
        "float16": np.array([-1.5, 0., 3.25], dtype="f2"),
        "float32_be": np.array([-1.5, 0., 3.25], dtype=">f4"),
        "float64": np.array([-1.5, 0., 3.25]),
        "complex": np.array([3+4j, -2-1j], dtype="c16"),
        "booleans": np.array([True, False]),
        "scalar": np.array(42),
        "empty": np.empty((3, 0, 2)),
        "structured": np.array([(1.5, 42), (-3., 7)], dtype=[("temperature", "f8"), ("count", ">i2")]),
        "nonfinite": np.array([np.nan, np.inf, -np.inf, 1.]),
        "object_unsupported": np.array({"setup": "pickle must not execute"}, dtype=object),
    }
    file = OUT / "compatibility.npz"
    np.savez_compressed(file, **fixtures)
    checks = 0
    for key, array in fixtures.items():
        if key == "object_unsupported":
            result = subprocess.run([str(BENCH), str(file), key, "--inspect"], capture_output=True)
            assert result.returncode != 0 and b"pickle" in result.stderr
            checks += 1
            continue
        result = probe(file, key)
        assert result["shape"] == list(array.shape), key
        actual = [v["value"] for v in result["values"]]
        expected = array.ravel(order="C")[:len(actual)]
        if array.dtype.kind in "iuf":
            for a, b in zip(actual, expected):
                if array.dtype.kind in "iu":
                    assert int(a) == int(b), (key, a, b)
                else:
                    assert float(a) == float(b) or np.isnan(float(a)) and np.isnan(b), key
        elif array.dtype.kind == "U":
            assert actual == [str(v) for v in expected], key
        elif array.dtype.kind == "b":
            assert actual == [str(bool(v)) for v in expected], key
        elif array.dtype.kind == "S":
            assert actual == [v.decode() for v in expected], key
        elif array.dtype.kind == "c":
            assert [complex(v.replace(" ", "")) for v in actual] == list(expected), key
        elif array.dtype.kind == "V":
            assert "temperature: 1.5" in actual[0] and "count: 42" in actual[0], key
        checks += 1
    # Bare NPY, including a real Fortran-order payload, independently of ZIP.
    for key in ["fortran", "cube", "unicode", "scalar"]:
        p = OUT / f"{key}.npy"
        np.save(p, fixtures[key])
        result = probe(p, key)
        assert result["shape"] == list(fixtures[key].shape)
        checks += 1
    report = {"numpy_compatibility_cases": checks}
    if args.recording:
        result = probe(args.recording.resolve(), "voltage", inspect=False)
        start = time.perf_counter()
        with np.load(args.recording, allow_pickle=False) as archive:
            array = archive["voltage"]
        report["numpy_load_ms"] = (time.perf_counter() - start) * 1000
        np.testing.assert_array_equal([float(v["value"]) for v in result["values"]], array[:10])
        counts, edges = np.histogram(array, bins=128)
        np.testing.assert_array_equal(result["exact"]["counts"], counts)
        np.testing.assert_allclose(result["exact"]["mean"], array.mean(), rtol=1e-10, atol=1e-16)
        np.testing.assert_allclose(result["exact"]["std"], array.std(), rtol=1e-10)
        assert result["exact"]["count"] == array.size
        result.pop("values", None)
        report["native_recording"] = result
        # Representative original rendering path: Matplotlib Agg, same 1M point cap.
        # This is a drawing-path comparison, not an end-to-end Qt application benchmark.
        from matplotlib.figure import Figure
        from matplotlib.backends.backend_agg import FigureCanvasAgg
        start = time.perf_counter()
        stride = (array.size + 999_999) // 1_000_000
        fig = Figure(figsize=(12, 6), dpi=100, constrained_layout=True)
        canvas = FigureCanvasAgg(fig)
        ax = fig.add_subplot(111)
        ax.plot(np.arange(0, array.size, stride), array[::stride], lw=1.2)
        canvas.draw()
        report["matplotlib_first_draw_ms"] = (time.perf_counter() - start) * 1000
        start = time.perf_counter()
        ax.set_xlim(100_000, 110_000)
        canvas.draw()
        report["matplotlib_zoom_draw_ms"] = (time.perf_counter() - start) * 1000
        report["matplotlib_display_points"] = len(array[::stride])
        report["recording_values_and_histogram_match_numpy"] = True
    (OUT / "numpy-validation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({k: v for k, v in report.items() if k != "native_recording"}, indent=2))


if __name__ == "__main__":
    main()
