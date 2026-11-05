import XCTest
import CoreGraphics
import LocalWaveDomain
@testable import localwave

@MainActor
final class FolderSortingTests: XCTestCase {
    func testDesktopFolderSortKeepsFoldersFirstAndRestoresOriginalFileOrder() {
        func path(_ id: Int64, _ name: String, directory: Bool = false) -> SourcePath {
            SourcePath(id: id, sourceId: 1, pathId: id, parentPathId: 1, name: name, relativePath: name, isDirectory: directory, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil)
        }
        let paths = [path(10, "Track 10.flac"), path(20, "Folder B", directory: true), path(2, "Track 2.flac"), path(30, "Folder A", directory: true), path(99, "cover.jpg")]
        XCTAssertEqual(WaveFolderView.sortPaths(paths, songs: [:], column: "Name", ascending: true).map(\.pathId), [20, 30, 2, 10, 99])
        XCTAssertEqual(WaveFolderView.sortPaths(paths, songs: [:], column: "Name", ascending: false).map(\.pathId), [20, 30, 10, 2, 99])
        XCTAssertEqual(WaveFolderView.sortPaths(paths, songs: [:], column: "", ascending: true).map(\.pathId), [20, 30, 10, 2, 99])
        XCTAssertEqual(WaveFolderView.sortPaths(paths, songs: [:], column: "Size", ascending: false).map(\.pathId), [20, 30, 10, 2, 99], "Equal numeric values must retain order in descending sorts")
    }
    func testContextTargetsPreserveDesktopBatchOrderAndNeverInventPhoneSelection() {
        func path(_ id: Int64, _ name: String, directory: Bool = false) -> SourcePath {
            SourcePath(id: id, sourceId: 1, pathId: id, parentPathId: 1, name: name, relativePath: name,
                isDirectory: directory, fileHashSHA256: nil, runId: 1, createdAt: Date(), updatedAt: nil)
        }
        func song(_ id: Int64?) -> Song {
            Song(id: id, songKey: id ?? 999, artist: "Artist", title: "Track", album: "Album", albumArtist: "Artist",
                 releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil,
                 pathHash: id ?? 999, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly)
        }
        let first = path(1, "First.flac"), second = path(2, "Second.flac")
        let directory = path(90, "Folder", directory: true), image = path(91, "cover.jpg")
        let missing = path(3, "Missing.flac"), unsaved = path(4, "Unsaved.flac"), alias = path(5, "Alias.flac")
        let ordered = [directory, second, image, first, missing, unsaved, alias]
        let cache = [Int64(1): song(101), 2: song(102), 4: song(nil), 5: song(102), 90: song(190), 91: song(191)]
        let selected: Set<Int64> = [1, 2, 3, 4, 5, 90, 91]
        let batch = WaveFolderView.contextTargets(clicked: first, orderedPaths: ordered, selected: selected, anchor: 2, cachedSongs: cache, wide: true)
        XCTAssertEqual(batch.selectedPathIds, selected)
        XCTAssertEqual(batch.anchor, 2)
        XCTAssertEqual(batch.songs.compactMap(\.id), [102, 101], "Display order survives unordered selection; only saved audio IDs appear once")
        let replacement = WaveFolderView.contextTargets(clicked: first, orderedPaths: ordered, selected: [2], anchor: 2, cachedSongs: cache, wide: true)
        XCTAssertEqual(replacement.selectedPathIds, [1])
        XCTAssertEqual(replacement.anchor, 1)
        XCTAssertEqual(replacement.songs.compactMap(\.id), [101])
        let phone = WaveFolderView.contextTargets(clicked: first, orderedPaths: ordered, selected: [2, 90], anchor: 2, cachedSongs: cache, wide: false)
        XCTAssertEqual(phone.selectedPathIds, [2, 90])
        XCTAssertEqual(phone.anchor, 2)
        XCTAssertEqual(phone.songs.compactMap(\.id), [101])
        let nonAudio = WaveFolderView.contextTargets(clicked: image, orderedPaths: ordered, selected: [1, 2], anchor: 2, cachedSongs: cache, wide: true)
        XCTAssertEqual(nonAudio.selectedPathIds, [1, 2])
        XCTAssertTrue(nonAudio.songs.isEmpty)
    }

    func testCompactFolderSelectionUsesRecursiveSavedAudioAndStableFileOrder() {
        func song(_ id: Int64?, _ path: String) -> Song {
            Song(id: id, songKey: id ?? 999, artist: "Artist", title: path, album: "Album", albumArtist: "Artist",
                 releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil,
                 pathHash: id ?? 999, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly,
                 sourceId: 1, relativePath: path)
        }
        let own = song(1, "Root/Z.flac"), childTwo = song(2, "Root/Child/Track 2.flac")
        let childTen = song(3, "Root/Child/Track 10.flac"), other = song(4, "Root/Other/Track.flac")
        let outside = song(5, "Outside.flac"), unsaved = song(nil, "Root/Unsaved.flac")
        let cache: [Int64: Song] = [11: own, 22: childTwo, 23: childTwo, 33: childTen, 44: other, 55: outside, 66: unsaved]
        let scope = WaveFolderView.compactSelectionScope(folderSongs: [other, childTen, childTwo, own, childTwo, unsaved],
            cachedSongs: cache, selectedPaths: [23, 33, 55, 66, 999], blocked: false)
        XCTAssertEqual(scope.pathIDs, [11, 22, 23, 33, 44], "Hidden descendants remain eligible; outside, missing and unsaved rows do not")
        XCTAssertEqual(scope.songs.compactMap(\.id), [1, 2, 3, 4], "Own files precede children; numeric file order is natural and duplicate IDs appear once")
        XCTAssertEqual(scope.selectedSongs.compactMap(\.id), [2, 3], "An alias selects its saved song once, preserving context order")
        let blocked = WaveFolderView.compactSelectionScope(folderSongs: [own, childTwo], cachedSongs: cache,
            selectedPaths: scope.pathIDs, blocked: true)
        XCTAssertTrue(blocked.pathIDs.isEmpty)
        XCTAssertTrue(blocked.selectedSongs.isEmpty)
        let retained = WaveFolderView.compactSelectionScope(folderSongs: [own, childTwo], cachedSongs: cache,
            selectedPaths: scope.pathIDs, blocked: true, availability: [11: .onDevice, 22: .unknown])
        XCTAssertEqual(retained.pathIDs, [11])
        XCTAssertEqual(retained.selectedSongs.compactMap(\.id), [1], "A verified readable cache remains usable when its source needs reconnecting")
    }

    func testColumnWidthsMatchMeasuredReferenceAutoLayout() {
        let preferred: [CGFloat] = [54, 147.96875, 78.078125, 66.40625, 71.546875, 56.8125]
        let columns = WaveFolderView.layoutColumns(preferred: preferred, available: 888)
        let measured: [CGFloat] = [54, 293.25, 154.73, 131.59, 141.80, 112.62]
        XCTAssertEqual(columns.reduce(0, +), 888, accuracy: 0.001)
        for (actual, expected) in zip(columns, measured) { XCTAssertEqual(actual, expected, accuracy: 0.03) }
    }

}
