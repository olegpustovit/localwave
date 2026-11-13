import AVFoundation
import CryptoKit
import Foundation
import LocalWaveDomain
import SQLite
import XCTest
@testable import LocalWaveData

final class WaveformCacheTests: XCTestCase {
    func testDecodedPeaksPersistAndRemainAvailableAfterAudioEviction() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("signal.caf")
        try writeSignal(url, secondHalf: 0.5)
        let song = fixture(url)
        let cache = WaveformCache(directory: directory.appendingPathComponent("cache"))
        let peaks = try await cache.peaks(for: song)
        XCTAssertEqual(peaks.count, 512)
        XCTAssertTrue(peaks.prefix(256).allSatisfy { $0 == 0 })
        XCTAssertTrue(peaks.suffix(256).allSatisfy { $0 > 0 && $0 <= 1 })
        try FileManager.default.removeItem(at: url)
        let reopened = WaveformCache(directory: directory.appendingPathComponent("cache"))
        let restored = try await reopened.peaks(for: song.copyWith(nil, .bookmarkOnly))
        XCTAssertEqual(restored, peaks)
    }

    func testChangedAudioInvalidatesCacheAndCorruptRowsRecover() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("signal.caf")
        let cacheDirectory = directory.appendingPathComponent("cache")
        let cache = WaveformCache(directory: cacheDirectory)
        try writeSignal(url, secondHalf: 0.5)
        let song = fixture(url)
        let initial = try await cache.peaks(for: song)
        XCTAssertTrue(initial.contains(1))
        try writeSignal(url, secondHalf: 0)
        try FileManager.default.setAttributes([.modificationDate: Date().addingTimeInterval(2)], ofItemAtPath: url.path)
        let silent = try await cache.peaks(for: song)
        XCTAssertEqual(silent, [Float](repeating: 0, count: 512))
        let database = try Connection(cacheDirectory.appendingPathComponent("waveforms.sqlite").path)
        try database.run("UPDATE waveforms SET peaks = ?", Blob(bytes: Array("invalid JSON".utf8)))
        let recovered = try await cache.peaks(for: song)
        XCTAssertEqual(recovered, silent)
    }

    func testConcurrentViewersShareValidCachedPeaks() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("signal.caf")
        try writeSignal(url, secondHalf: 0.5)
        let song = fixture(url)
        let cache = WaveformCache(directory: directory.appendingPathComponent("cache"))
        let values = try await withThrowingTaskGroup(of: [Float].self) { group in
            for _ in 0..<8 { group.addTask { try await cache.peaks(for: song) } }
            var values: [[Float]] = []
            for try await peaks in group { values.append(peaks) }
            return values
        }
        XCTAssertEqual(values.count, 8)
        XCTAssertTrue(values.allSatisfy { $0 == values[0] && $0.count == 512 })
        let database = try Connection(directory.appendingPathComponent("cache/waveforms.sqlite").path)
        XCTAssertEqual(try database.scalar("SELECT COUNT(*) FROM waveforms") as? Int64, 1)
    }

    func testLRUTouchKeepsContentAddressedWaveformVersion() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let music = directory.appendingPathComponent("Music")
        try FileManager.default.createDirectory(at: music, withIntermediateDirectories: true)
        let original = music.appendingPathComponent("signal.caf")
        try writeSignal(original, secondHalf: 0.5)
        let hash = SHA256.hash(data: try Data(contentsOf: original)).map { String(format: "%02x", $0) }.joined()
        let audio = music.appendingPathComponent(hash + ".caf")
        try FileManager.default.moveItem(at: original, to: audio)
        let cacheDirectory = directory.appendingPathComponent("cache")
        let cache = WaveformCache(directory: cacheDirectory, audioCacheDirectory: music)
        let initial = try await cache.peaks(for: fixture(audio))
        let database = try Connection(cacheDirectory.appendingPathComponent("waveforms.sqlite").path)
        let signature = try database.scalar("SELECT source_signature FROM waveforms") as? String
        try FileManager.default.setAttributes([.modificationDate: Date().addingTimeInterval(5)], ofItemAtPath: audio.path)
        let afterTouch = try await cache.peaks(for: fixture(audio))
        XCTAssertEqual(afterTouch, initial)
        XCTAssertEqual(try database.scalar("SELECT source_signature FROM waveforms") as? String, signature)
    }

    func testBassHeavySignalRetainsChangesAboveBass() async throws {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString + ".caf")
        defer { try? FileManager.default.removeItem(at: url) }
        let format = try XCTUnwrap(AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 1))
        let buffer = try XCTUnwrap(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 48000))
        buffer.frameLength = 48000
        let samples = try XCTUnwrap(buffer.floatChannelData?[0])
        for index in 0..<48000 {
            let time = Double(index) / 48000
            let detail = index < 24000 ? 0.02 : 0.25
            samples[index] = Float(0.7 * sin(2 * .pi * 50 * time) + detail * sin(2 * .pi * 2000 * time))
        }
        let file = try AVAudioFile(forWriting: url, settings: format.settings)
        try file.write(from: buffer)
        let peaks = try await WaveformCache.decode(url: url)
        XCTAssertEqual(peaks.count, 512)
        XCTAssertTrue(peaks.allSatisfy { $0.isFinite && (0...1).contains($0) })
        let quietDetail = peaks[32..<224].reduce(0, +) / 192
        let loudDetail = peaks[288..<480].reduce(0, +) / 192
        XCTAssertGreaterThan(loudDetail, quietDetail * 1.2)
    }

    private func fixture(_ url: URL) -> Song {
        Song(id: 1, songKey: 1, artist: "Test", title: "Signal", album: "Test", albumArtist: "Test", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 1, createdAt: Date(), updatedAt: nil, localFilePath: url.path, fileState: .copied)
    }

    private func writeSignal(_ url: URL, secondHalf: Float) throws {
        let format = try XCTUnwrap(AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 1))
        let buffer = try XCTUnwrap(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 48000))
        buffer.frameLength = 48000
        let samples = try XCTUnwrap(buffer.floatChannelData?[0])
        for index in 0..<48000 { samples[index] = index < 24000 ? 0 : secondHalf }
        let file = try AVAudioFile(forWriting: url, settings: format.settings)
        try file.write(from: buffer)
    }
}
