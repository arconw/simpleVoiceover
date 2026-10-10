import argparse
import json
import math
import unittest
from unittest.mock import patch

import audio
import lab
import measurements
import regression


class ConfigurationTests(unittest.TestCase):
    def test_native_formats_cannot_be_mixed_with_other_distributions(self):
        with self.assertRaisesRegex(ValueError, "Native package format"):
            lab.profile("ubuntu22-pulse", argparse.Namespace(artifact="rpm"))

    def test_bundled_webkit_cannot_claim_a_system_version_pin(self):
        with self.assertRaisesRegex(ValueError, "contains its own WebKit version"):
            lab.profile("ubuntu22-appimage", argparse.Namespace(webkit_version="2.42.5"))

    def test_runtime_variants_share_images_but_have_distinct_sessions(self):
        native = lab.profile("ubuntu22-pulse", argparse.Namespace())
        bundled = lab.profile("ubuntu22-pulse", argparse.Namespace(artifact="appimage"))
        self.assertEqual(lab.image_name(native), lab.image_name(bundled))
        self.assertNotEqual(lab.instance(native, "auto"), lab.instance(bundled, "auto"))


class HostBridgeTests(unittest.TestCase):
    def test_legacy_pulse_streams_follow_defaults_without_moving_unrelated_clients(self):
        identifier = "dev.simplevoiceover.linux-testbench.example"
        streams = [
            {"client": "20", "sink": "3", "properties": {"application.id": identifier}},
            {"client": "10", "sink": "3", "properties": {"application.id": "browser"}},
            {"client": "30", "sink": "9", "properties": {"application.id": identifier}},
        ]
        answers = {
            ("get-default-sink",): "speaker",
            ("list", "short", "sinks"): "9\tspeaker\tPipeWire",
            ("--format=json", "list", "sink-inputs"): json.dumps(streams),
            (
                "list",
                "short",
                "sink-inputs",
            ): "101\t3\t10\tprotocol\n102\t3\t20\tprotocol\n103\t9\t30\tprotocol",
            ("get-default-source",): "microphone",
            ("list", "short", "sources"): "12\tmicrophone\tPipeWire",
            ("--format=json", "list", "source-outputs"): "[]",
            ("list", "short", "source-outputs"): "",
        }
        moves = []

        def respond(*arguments, server):
            self.assertEqual(server, audio.HOST_SERVER)
            if arguments[0].startswith("move-"):
                moves.append(arguments)
                return ""
            return answers[arguments]

        with patch.object(audio, "pactl", side_effect=respond):
            audio.follow_host_defaults(identifier)
        self.assertEqual(moves, [("move-sink-input", "102", "speaker")])


class MeasurementTests(unittest.TestCase):
    def test_source_tones_remain_distinct_across_recording_phase_discontinuities(self):
        samples = [
            0.2 * math.sin(2 * math.pi * 440 * index / 16000 + (index // 3680) * math.pi)
            for index in range(32000)
        ]
        self.assertGreater(measurements.tone_amplitude(samples, 16000, 440), 0.15)
        self.assertLess(measurements.tone_amplitude(samples, 16000, 660), 0.01)

    def test_silence_cannot_pass_as_an_expected_microphone(self):
        self.assertEqual(measurements.tone_amplitude([0] * 32000, 16000, 440), 0)


class RegressionTests(unittest.TestCase):
    def test_focused_run_cannot_claim_full_coverage_after_failed_startup_baseline(self):
        baseline = {
            "preset": {"tests": [{"name": "desktop.start", "status": "failed", "required": True}]}
        }
        current = {
            "preset": {
                "selection": "desktop.start",
                "tests": [{"name": "desktop.start", "status": "passed", "required": True}],
            }
        }
        result = regression.compare(baseline, current)
        self.assertEqual(
            result["regressions"],
            [{"profile": "preset", "test": "coverage.must-be-complete", "status": "partial"}],
        )

    def test_skipped_or_missing_previously_passing_checks_block_comparison(self):
        baseline = {
            "preset": {
                "tests": [
                    {"name": "seek", "status": "passed", "required": True},
                    {"name": "record", "status": "passed", "required": True},
                    {"name": "mkv", "status": "failed", "required": True},
                ]
            }
        }
        current = {
            "preset": {
                "tests": [
                    {"name": "seek", "status": "skipped", "required": True},
                    {"name": "mkv", "status": "passed", "required": True},
                    {"name": "effects", "status": "failed", "required": True},
                ]
            }
        }
        result = regression.compare(baseline, current)
        self.assertEqual(result["improvements"], [{"profile": "preset", "test": "mkv"}])
        self.assertEqual(
            result["regressions"],
            [
                {"profile": "preset", "test": "seek", "status": "skipped"},
                {"profile": "preset", "test": "record", "status": "missing"},
                {"profile": "preset", "test": "effects", "status": "failed"},
            ],
        )


if __name__ == "__main__":
    unittest.main()
