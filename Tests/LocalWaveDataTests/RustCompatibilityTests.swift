import Foundation
import AVFoundation
import CryptoKit
import Darwin
import XCTest
import SQLite
import LocalWaveCore
import LocalWaveDomain
@testable import LocalWaveData

final class RustCompatibilityTests: XCTestCase {
    func testSwiftAndRustExchangeEveryTypedPlaybackContext() throws {
        guard let library = ProcessInfo.processInfo.environment["LOCALWAVE_RUST_BOOKMARKS"] else {
            throw XCTSkip("Run tools/check-database-compatibility.sh for typed context interop")
        }
        let handle = try XCTUnwrap(dlopen(library, RTLD_NOW | RTLD_LOCAL))
        defer { dlclose(handle) }
        typealias Exchange = @convention(c) (UnsafePointer<UInt8>?, Int, UnsafeMutablePointer<UInt8>?, Int) -> Int
        let exchange = unsafeBitCast(try XCTUnwrap(dlsym(handle, "lw_context_interop_fixture")), to: Exchange.self)
        let name = "名前\u{0000}\"\\\n🦊"
        let contexts: [PlaybackContext] = [
            .library, .album(name: name, artist: "Björk"), .artist(name),
            .playlist(id: .max, name: name), .playlist(id: .min, name: ""),
            .folder(sourceID: .max, pathID: .min, name: name),
            .folder(sourceID: -1, pathID: nil, name: "Root"), .automatic(name), .selection(name)
        ]
        for context in contexts {
            let encoded = try JSONEncoder().encode(context)
            var output = [UInt8](repeating: 0, count: 4096)
            let length = encoded.withUnsafeBytes { input in output.withUnsafeMutableBufferPointer { out in
                exchange(input.bindMemory(to: UInt8.self).baseAddress, encoded.count, out.baseAddress, out.count)
            } }
            XCTAssertGreaterThan(length, 0)
            guard length > 0 else { continue }
            XCTAssertEqual(try JSONDecoder().decode(PlaybackContext.self, from: Data(output.prefix(length))), context)
        }
        for json in [
            #"{"playlist":{"id":1.0,"name":"one"}}"#,
            #"{"playlist":{"id":9223372036854775807,"name":"max"}}"#,
            #"{"playlist":{"id":9223372036854775807.0,"name":"max"}}"#,
            #"{"playlist":{"id":-9223372036854775809,"name":"too small"}}"#,
            #"{"playlist":{"id":9223372036854775808,"name":"too big"}}"#,
            #"{"playlist":{"id":true,"name":"boolean"}}"#,
            #"{"folder":{"sourceID":1,"pathID":null,"name":"root"}}"#,
            #"{"library":{},"future":{}}"#
        ] {
            let encoded = Data(json.utf8)
            let expected = try? JSONDecoder().decode(PlaybackContext.self, from: encoded)
            var output = [UInt8](repeating: 0, count: 4096)
            let length = encoded.withUnsafeBytes { input in output.withUnsafeMutableBufferPointer { out in
                exchange(input.bindMemory(to: UInt8.self).baseAddress, encoded.count, out.baseAddress, out.count)
            } }
            if let expected {
                XCTAssertGreaterThan(length, 0, json)
                if length > 0 { XCTAssertEqual(try JSONDecoder().decode(PlaybackContext.self, from: Data(output.prefix(length))), expected, json) }
            } else {
                XCTAssertLessThan(length, 0, json)
            }
        }
    }

    func testNativeRustPlaybackHistoryPreservesSwiftCountsAndDates() throws {
        guard let library = ProcessInfo.processInfo.environment["LOCALWAVE_RUST_BOOKMARKS"] else {
            throw XCTSkip("Run tools/check-database-compatibility.sh for history interop")
        }
        let handle = try XCTUnwrap(dlopen(library, RTLD_NOW | RTLD_LOCAL))
        defer { dlclose(handle) }
        typealias Exchange = @convention(c) (UnsafePointer<CChar>?) -> Int32
        let exchange = unsafeBitCast(try XCTUnwrap(dlsym(handle, "lw_history_interop_fixture")), to: Exchange.self)
        let suite = "com.olegpustovit.localwave.history-interop-" + UUID().uuidString
        let preferences = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { preferences.removePersistentDomain(forName: suite) }
        preferences.set([String(Int64.max): 7, "22": 3, "88": Int.max, "77": -5], forKey: "lwPlayCounts")
        preferences.set([String(Int64.max): 100.125, "22": 101.25], forKey: "lwLastPlayed")
        XCTAssertEqual(suite.withCString { exchange($0) }, 0)
        XCTAssertEqual(preferences.dictionary(forKey: "lwPlayCounts") as? [String: Int], [String(Int64.max): 8, "22": 3, "42": 1, "88": Int.max, "77": 1])
        XCTAssertEqual(preferences.dictionary(forKey: "lwLastPlayed") as? [String: Double], [String(Int64.max): 200.375, "22": 101.25, "42": 201.625, "88": 202.75, "77": 203.875])
    }

    // Native decoder compatibility runs only in the explicit interop gate, outside the fast shared loop.
    func testNativeRustWaveformSignaturesAndPCMMatchSwift() async throws {
        guard let library = ProcessInfo.processInfo.environment["LOCALWAVE_RUST_BOOKMARKS"] else { throw XCTSkip("Run tools/check-database-compatibility.sh for waveform interop") }
        let handle = try XCTUnwrap(dlopen(library, RTLD_NOW | RTLD_LOCAL))
        defer { dlclose(handle) }
        typealias Extract = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafeMutablePointer<UInt8>?, UnsafeMutablePointer<Float>?) -> Int32
        let extract = unsafeBitCast(try XCTUnwrap(dlsym(handle, "lw_waveform_interop_fixture")), to: Extract.self)
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let music = directory.appendingPathComponent("Music", isDirectory: true)
        try FileManager.default.createDirectory(at: music, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let signal = directory.appendingPathComponent("名前 signal.caf")
        let format = try XCTUnwrap(AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 2))
        let buffer = try XCTUnwrap(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 48000))
        buffer.frameLength = 48000
        for frame in 0..<48000 {
            buffer.floatChannelData?[0][frame] = frame < 12000 ? 0 : 0.25
            buffer.floatChannelData?[1][frame] = frame < 24000 ? 0 : -0.5
        }
        do { let file = try AVAudioFile(forWriting: signal, settings: format.settings); try file.write(from: buffer) }
        let hash = SHA256.hash(data: try Data(contentsOf: signal)).map { String(format: "%02x", $0) }.joined()
        let cached = music.appendingPathComponent(hash + ".caf")
        try FileManager.default.copyItem(at: signal, to: cached)
        let aac = try XCTUnwrap(Bundle.module.url(forResource: "tagged-m4a", withExtension: "m4a", subdirectory: "Fixtures"))
        let flac = try XCTUnwrap(Bundle.module.url(forResource: "tagged", withExtension: "flac", subdirectory: "Fixtures"))
        for file in [signal, cached, aac, flac] {
            let swift = try await WaveformCache.decode(url: file)
            let temporary = file == signal || file == cached
            for timestamp in temporary ? [0.0, 0.000001, 100.125, 1794513600.1234567] : [0.0] {
                // The bundled fixture remains untouched; temporary files exercise Double spelling.
                if temporary { try FileManager.default.setAttributes([.modificationDate: Date(timeIntervalSince1970: timestamp)], ofItemAtPath: file.path) }
                let attributes = try FileManager.default.attributesOfItem(atPath: file.path)
                let version = file == cached ? hash : file.standardizedFileURL.path + "|" + String((attributes[.modificationDate] as? Date)?.timeIntervalSince1970 ?? 0)
                let expected = Array(SHA256.hash(data: Data((version + "|" + String(describing: attributes[.size])).utf8)))
                let (result, digest, rust) = await Task.detached {
                    var digest = [UInt8](repeating: 0, count: 32), peaks = [Float](repeating: 0, count: 512)
                    let result = file.path.withCString { path in music.path.withCString { root in digest.withUnsafeMutableBufferPointer { out in peaks.withUnsafeMutableBufferPointer { values in extract(path, root, out.baseAddress, values.baseAddress) } } } }
                    return (result, digest, peaks)
                }.value
                XCTAssertEqual(result, 0)
                guard result == 0 else { continue }
                XCTAssertEqual(digest, expected, file.path)
                XCTAssertEqual(swift.count, rust.count)
                for (expected, actual) in zip(swift, rust) { XCTAssertEqual(actual, expected, accuracy: 0.000002, file.path) }
            }
        }
    }

    func testSwiftAndRustExchangeNativePlayerPreferencesAndOpaquePlaybackContext() throws {
        guard let library = ProcessInfo.processInfo.environment["LOCALWAVE_RUST_BOOKMARKS"] else {
            throw XCTSkip("Run tools/check-database-compatibility.sh for native preferences interop")
        }
        let handle = try XCTUnwrap(dlopen(library, RTLD_NOW | RTLD_LOCAL))
        defer { dlclose(handle) }
        typealias Exchange = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<UInt8>?, Int) -> Int32
        let exchange = unsafeBitCast(try XCTUnwrap(dlsym(handle, "lw_preferences_interop_fixture")), to: Exchange.self)
        let suite = "LocalWaveRustPreferences-" + UUID().uuidString
        let preferences = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { preferences.removePersistentDomain(forName: suite) }
        let context = PlaybackContext.album(name: "名前\u{0000}", artist: "Björk")
        let data = try JSONEncoder().encode(context)
        preferences.set([Int64.max, 22, 22, -1] as [Int64], forKey: "currentQueue")
        preferences.set(2, forKey: "currentQueueIndex")
        preferences.set(Float(0.625), forKey: "playerVolume")
        preferences.set(data, forKey: "lwPlaybackContext")
        preferences.set("light", forKey: "lwAppearance")
        preferences.set("green", forKey: "lwSignal")
        preferences.set(4, forKey: "lwCacheGB")
        preferences.set(false, forKey: "lwDownloadAhead")
        let result = suite.withCString { key in data.withUnsafeBytes { bytes in exchange(key, bytes.bindMemory(to: UInt8.self).baseAddress, data.count) } }
        XCTAssertEqual(result, 0)
        XCTAssertEqual(preferences.array(forKey: "currentQueue") as? [Int64], [22, Int64.max, 22])
        XCTAssertEqual(preferences.integer(forKey: "currentQueueIndex"), 2)
        XCTAssertNotNil(preferences.object(forKey: "playerVolume"))
        XCTAssertEqual(preferences.float(forKey: "playerVolume"), 0)
        XCTAssertEqual(try JSONDecoder().decode(PlaybackContext.self, from: XCTUnwrap(preferences.data(forKey: "lwPlaybackContext"))), context)
        XCTAssertEqual(preferences.string(forKey: "lwAppearance"), "dark")
        XCTAssertEqual(preferences.string(forKey: "lwSignal"), "mono")
        XCTAssertEqual(preferences.integer(forKey: "lwCacheGB"), 16)
        XCTAssertTrue(preferences.bool(forKey: "lwReduceMotion"))
    }
    func testNativeRustAndSwiftBookmarksResolveAfterPhysicalFolderRename() throws {
        guard let library = ProcessInfo.processInfo.environment["LOCALWAVE_RUST_BOOKMARKS"] else {
            throw XCTSkip("Run tools/check-database-compatibility.sh for native bookmark interop")
        }
        let handle = try XCTUnwrap(dlopen(library, RTLD_NOW | RTLD_LOCAL))
        defer { dlclose(handle) }
        typealias Resolve = @convention(c) (UnsafePointer<UInt8>?, Int, UnsafeMutablePointer<UInt8>?, Int) -> Int
        typealias Create = @convention(c) (UnsafePointer<CChar>?, UnsafeMutablePointer<UInt8>?, Int) -> Int
        let resolve = unsafeBitCast(try XCTUnwrap(dlsym(handle, "lw_bookmark_interop_resolve_fixture")), to: Resolve.self)
        let create = unsafeBitCast(try XCTUnwrap(dlsym(handle, "lw_bookmark_interop_create")), to: Create.self)
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let original = directory.appendingPathComponent("Original", isDirectory: true)
        let renamed = directory.appendingPathComponent("Renamed", isDirectory: true)
        try FileManager.default.createDirectory(at: original, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        try Data([1, 2, 3]).write(to: original.appendingPathComponent("swift.flac"))
        let bookmark = try original.bookmarkData(options: sourceBookmarkCreationOptions)
        try FileManager.default.moveItem(at: original, to: renamed)
        var output = [UInt8](repeating: 0, count: 4096)
        let resolvedLength = bookmark.withUnsafeBytes { bytes in
            output.withUnsafeMutableBufferPointer { out in resolve(bytes.bindMemory(to: UInt8.self).baseAddress, bookmark.count, out.baseAddress, out.count) }
        }
        XCTAssertGreaterThan(resolvedLength, 0)
        guard resolvedLength > 0 else { return }
        let resolvedPath = try XCTUnwrap(String(bytes: output.prefix(resolvedLength), encoding: .utf8))
        XCTAssertEqual(URL(fileURLWithPath: resolvedPath).resolvingSymlinksInPath(), renamed.resolvingSymlinksInPath())
        var replacement = [UInt8](repeating: 0, count: 65536)
        let createdLength = renamed.path.withCString { path in
            replacement.withUnsafeMutableBufferPointer { out in create(path, out.baseAddress, out.count) }
        }
        XCTAssertGreaterThan(createdLength, 0)
        guard createdLength > 0 else { return }
        var stale = false
        let restored = try URL(resolvingBookmarkData: Data(replacement.prefix(createdLength)), options: sourceBookmarkResolutionOptions, bookmarkDataIsStale: &stale)
        let scoped = restored.startAccessingSecurityScopedResource()
        defer { if scoped { restored.stopAccessingSecurityScopedResource() } }
        XCTAssertEqual(restored.resolvingSymlinksInPath(), renamed.resolvingSymlinksInPath())
        XCTAssertEqual(try Data(contentsOf: restored.appendingPathComponent("swift.flac")), Data([1, 2, 3]))
    }

    func testSwiftAndRustExchangeDatabaseWithoutMigration() async throws {
        guard let executable = ProcessInfo.processInfo.environment["LOCALWAVE_RUST_EXCHANGE"] else {
            throw XCTSkip("Run tools/check-database-compatibility.sh for the Rust exchange gate")
        }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let path = directory.appendingPathComponent("musicApp29.sqlite").path
        let db = try Connection(path)
        let songs = try SQLiteSongRepository(db: db)
        let sources = try SQLiteSourceRepository(db: db)
        let paths = try SQLiteSourcePathRepository(db: db)
        let pathSearch = try SQLiteSourcePathSearchRepository(db: db)
        let users = try SQLiteUserRepository(db: db)
        let user = try await users.create(user: User(id: nil, icloudId: 12345))
        let userID = try XCTUnwrap(user.id)
        let source = try await sources.create(source: Source(id: nil, dirPath: directory.path, pathId: makeURLHash(directory), userId: userID, type: .iCloud,
            totalPaths: nil, syncError: "Needs reconnect", isCurrent: true, createdAt: Date(timeIntervalSince1970: 100.125), lastSyncedAt: nil, updatedAt: nil, bookmark: Data([1, 2, 3])))
        let sourceID = try XCTUnwrap(source.id)
        let lists = try SQLitePlaylistRepository(db: db)
        let links = try SQLitePlaylistSongRepository(db: db)
        let original = try await songs.upsertSong(Song(id: nil, songKey: generateSongKey(artist: "Swift", title: "Swift fixture", album: "Exchange"),
            artist: "Swift", title: "Swift fixture", album: "Exchange", albumArtist: "Various", releaseYear: 2026, discNumber: 2, trackNumber: 3,
            coverArtPath: nil, bookmark: Data([0, 255, 3]), pathHash: hashStringToInt64("file:///fixture/swift.flac"),
            createdAt: Date(timeIntervalSince1970: 100), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly,
            sourceId: sourceID, relativePath: "swift.flac", duration: 90.5, fileSize: 1000, fileExtension: "flac", sampleRate: 48000, bitDepth: 24, sourceAvailability: .onDevice))
        let sourcePath = SourcePath(id: nil, sourceId: sourceID, pathId: makeURLHash(directory.appendingPathComponent("swift.flac")), parentPathId: source.pathId,
            name: "swift.flac", relativePath: "swift.flac", isDirectory: false, fileHashSHA256: Data([0, 255]), runId: 1, createdAt: Date(timeIntervalSince1970: 100.125), updatedAt: nil, fileSize: 1000)
        try await paths.batchUpsert(paths: [sourcePath])
        try await pathSearch.batchUpsertIntoFTS(paths: [sourcePath])
        let id = try XCTUnwrap(original.id)
        try await songs.toggleSongAsFavorite(songId: id, state: true)
        let list = try await lists.create(playlist: Playlist(id: nil, name: "Preserve membership", createdAt: Date(timeIntervalSince1970: 100.125), updatedAt: nil))
        let listID = try XCTUnwrap(list.id)
        try await links.addSong(playlistId: listID, songId: id)
        try db.execute("CREATE TABLE rust_compat_keys (text TEXT, hash INTEGER, songKey INTEGER, path TEXT, pathId INTEGER)")
        let values = ["", "hello", "Björk", "é", "e\u{301}", "İSTANBUL", "ΟΣ", "日本語", "العربية", "🎧", "A__B", "a;+:@!$&()=,~[]'", "a/../b", "a?b#c"]
        for value in values {
            let filePath = "/Music/\(value)/track.flac"
            try db.run("INSERT INTO rust_compat_keys VALUES (?,?,?,?,?)", value, hashStringToInt64(value), generateSongKey(artist: value, title: value, album: value), filePath, makeURLHash(URL(fileURLWithPath: filePath)))
        }
        try db.execute("CREATE TABLE rust_compat_titles (fileName TEXT, title TEXT)")
        for name in ["01 – COUSINS.m4a", "07.artist-name.flac", "1984.m4a", "CJKタイトル.wav", "١٢ – عربي.flac", "१२ - Title.wav", "１２. 全角.mp3", "Ⅻ – Roman.flac", "² – Superscript.flac", "01 - .flac", "1234-Title.mp3", "  001 _ Track.flac", ".flac", "music.v1/07.title.FLAC"] {
            try db.run("INSERT INTO rust_compat_titles VALUES (?,?)", name, DefaultSongImportService.fallbackTitle(for: name))
        }
        let waveforms = try Connection(directory.appendingPathComponent("waveforms.sqlite").path)
        try waveforms.execute("CREATE TABLE waveforms(song_key INTEGER PRIMARY KEY,source_signature TEXT NOT NULL,peaks BLOB NOT NULL)")
        var peaks = (0..<512).map { Float($0) / 511 }
        peaks[0] = -0.0; peaks[1] = Float.leastNonzeroMagnitude; peaks[2] = 1e-30
        try waveforms.run("INSERT INTO waveforms VALUES (?,?,?)", original.songKey, "swift-waveform", Blob(bytes: Array(try JSONEncoder().encode(peaks))))
        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = [path]
        try process.run()
        process.waitUntilExit()
        XCTAssertEqual(process.terminationStatus, 0)
        guard process.terminationStatus == 0 else { return }
        let signature = try waveforms.scalar("SELECT source_signature FROM waveforms WHERE song_key=?", original.songKey) as? String
        XCTAssertEqual(signature, "rust-waveform")
        for row in try waveforms.prepare("SELECT peaks FROM waveforms WHERE song_key=?", original.songKey) {
            let blob = try XCTUnwrap(row[0] as? Blob)
            let restored = try JSONDecoder().decode([Float].self, from: Data(blob.bytes))
            XCTAssertEqual(restored, Array(peaks.reversed()))
        }
        let updatedRows = await songs.getSongs(ids: [id])
        let updated = try XCTUnwrap(updatedRows.first)
        XCTAssertEqual(updated.title, "Edited by Rust")
        XCTAssertEqual(updated.isFavorite, 1)
        XCTAssertEqual(updated.duration, 90.5)
        let membership = try await links.getSongs(playlistId: listID)
        XCTAssertEqual(membership.compactMap(\.id), [id])
        let rustRows = try await songs.searchSongsFTS(query: "Rust fixture", limit: 20, offset: 0)
        let rust = try XCTUnwrap(rustRows.first)
        XCTAssertEqual(rust.artist, "Rust")
        XCTAssertEqual(rust.albumArtist, "Various")
        XCTAssertEqual(rust.bookmark, Data([9, 0, 254]))
        XCTAssertEqual(rust.fileState, .copied)
        XCTAssertEqual(rust.localFilePath, "Music/rust.flac")
        XCTAssertEqual(rust.isFavorite, 1)
        XCTAssertEqual(rust.duration, 42.25)
        XCTAssertEqual(rust.sampleRate, 96000)
        XCTAssertEqual(rust.bitDepth, 24)
        XCTAssertEqual(rust.sourceAvailability, .onDevice)
        let rustLists = try await lists.getAll()
        let rustList = try XCTUnwrap(rustLists.first { $0.name == "Rust list" })
        XCTAssertEqual(rustList.createdAt.timeIntervalSince1970, 200.375, accuracy: 0.001)
        XCTAssertEqual(rustList.updatedAt?.timeIntervalSince1970 ?? -1, 201.625, accuracy: 0.001)
        let rustMembership = try await links.getSongs(playlistId: XCTUnwrap(rustList.id))
        XCTAssertEqual(rustMembership.compactMap(\.id), [id, rust.id].compactMap { $0 })
        let resolvedSourceRow = try await sources.getOne(id: sourceID)
        let resolvedSource = try XCTUnwrap(resolvedSourceRow)
        XCTAssertEqual(resolvedSource.bookmark, Data([9, 8, 7]))
        XCTAssertNil(resolvedSource.syncError)
        XCTAssertEqual(resolvedSource.createdAt.timeIntervalSince1970, 100.125, accuracy: 0.001)
        let rustPathRow = try await paths.getByPath(relativePath: "rust.flac", sourceId: sourceID)
        let rustPath = try XCTUnwrap(rustPathRow)
        XCTAssertEqual(rustPath.fileSize, 1024)
        XCTAssertEqual(rustPath.createdAt.timeIntervalSince1970, 200.375, accuracy: 0.001)
        let pathMatches = try await pathSearch.search(sourceId: sourceID, query: "rust", limit: 10)
        XCTAssertEqual(pathMatches.map(\.pathId), [rustPath.pathId])
    }
}
