import Foundation
import Testing
import SwiftUI
import LocalWaveDomain
@testable import localwave

struct WaveLibraryTests {
    private func song(state: FileState = .copied, favorite: Int64 = 1) -> Song {
        Song(id: 1, songKey: 1, artist: "Test Artist", title: "Night Air", album: "Weather", albumArtist: "Test Artist", releaseYear: 2019, discNumber: nil, trackNumber: 1, coverArtPath: nil, bookmark: nil, pathHash: 1, createdAt: Date(), updatedAt: nil, localFilePath: "music/night.FLAC", fileState: state, isFavorite: favorite)
    }

    @MainActor @Test func playlistRemovalClearsOnlyAfterSuccessAndUndoDoesNotReselect() async throws {
        let tracks = (1...3).map { number in
            Song(id: Int64(number), songKey: Int64(number), artist: "Artist", title: "Track \(number)", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: Int64(number), createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly)
        }
        let repository = RemovalTestRepository(tracks)
        let model = WaveLibraryModel(try DependencyContainer(), playlistSongRepository: repository)
        func select() {
            model.listSongs[77] = tracks
            model.selected = [1, 2]
            model.selecting = true
            model.selectionAnchor = 1
        }

        select()
        await model.remove([], from: 77)
        #expect(model.selected == [1, 2] && model.selecting)
        #expect(model.undoRemoval == nil)

        await repository.failRemoval(of: 1)
        await model.remove([1, 2], from: 77)
        #expect(model.selected == [1, 2] && model.selecting)
        #expect(model.selectionAnchor == 1)
        #expect(model.undoRemoval == nil)
        #expect(model.error != nil)
        #expect(try await repository.getSongs(playlistId: 77).map(\.uniqueId) == [1, 2, 3])

        select()
        await repository.failRemoval(of: 2)
        await model.remove([1, 2], from: 77)
        #expect(model.selected.isEmpty && !model.selecting)
        #expect(model.selectionAnchor == nil)
        #expect(model.undoRemoval?.songIDs == [1])
        #expect(try await repository.getSongs(playlistId: 77).map(\.uniqueId) == [2, 3])
        #expect(await model.undoRemove())
        #expect(try await repository.getSongs(playlistId: 77).map(\.uniqueId) == [1, 2, 3])
        #expect(model.selected.isEmpty && !model.selecting)
        #expect(model.undoRemoval == nil)

        select()
        await repository.failRemoval(of: nil)
        await model.remove([1, 2], from: 77)
        #expect(model.selected.isEmpty && !model.selecting)
        #expect(model.selectionAnchor == nil)
        #expect(model.undoRemoval?.songIDs == [1, 2])
        #expect(try await repository.getSongs(playlistId: 77).map(\.uniqueId) == [3])
        #expect(await model.undoRemove())
        #expect(try await repository.getSongs(playlistId: 77).map(\.uniqueId) == [1, 2, 3])
        #expect(model.selected.isEmpty && !model.selecting)
    }

    @Test func playlistDragMovesIntoTargetPositionInBothDirectionsWithoutDroppingTracks() {
        let tracks = (1...4).map { number in
            Song(id: Int64(number), songKey: Int64(number), artist: "Artist", title: "Track \(number)", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: Int64(number), createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly)
        }
        #expect(wavePlaylistSongsMoving(tracks, draggedID: 1, targetID: 3)?.map(\.uniqueId) == [2, 3, 1, 4])
        #expect(wavePlaylistSongsMoving(tracks, draggedID: 4, targetID: 2)?.map(\.uniqueId) == [1, 4, 2, 3])
        #expect(wavePlaylistSongsMoving(tracks, draggedID: 1, targetID: 4)?.map(\.title) == ["Track 2", "Track 3", "Track 4", "Track 1"])
        #expect(wavePlaylistSongsMoving(tracks, draggedID: 2, targetID: 2) == nil)
        #expect(wavePlaylistSongsMoving(tracks, draggedID: 99, targetID: 2) == nil)
    }

    @Test func desktopPlaylistGroupDropPreservesDisplayedGroupOrderAndInsertsBeforeTarget() {
        let tracks = (1...5).map { number in
            Song(id: Int64(number), songKey: Int64(number), artist: "Artist", title: "Track \(number)", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: Int64(number), createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly)
        }
        #expect(wavePlaylistSongsMovingGroup(tracks, draggedIDs: [1, 3], before: 5)?.map(\.uniqueId) == [2, 4, 1, 3, 5])
        #expect(wavePlaylistSongsMovingGroup(tracks, draggedIDs: [5, 3], before: 2)?.map(\.uniqueId) == [1, 5, 3, 2, 4])
        #expect(wavePlaylistSongsMovingGroup(tracks, draggedIDs: [1], before: 3)?.map(\.uniqueId) == [2, 1, 3, 4, 5])
        #expect(wavePlaylistSongsMovingGroup(tracks, draggedIDs: [3, 1, 3, 99], before: 1)?.map(\.uniqueId) == [2, 4, 5, 3, 1])
        #expect(wavePlaylistSongsMovingGroup(tracks, draggedIDs: [99], before: 2) == nil)
        #expect(wavePlaylistSongsMovingGroup(tracks, draggedIDs: [2, 3], before: 4) == nil)
    }

    @Test func findCombinesTextMetadataAndAvailability() {
        #expect(WaveSongQuery("night offline flac year:2019 favorites").matches(song(), availability: .onDevice))
        #expect(!WaveSongQuery("night cloud").matches(song()))
        #expect(!WaveSongQuery("night offline").matches(song(state: .bookmarkOnly)))
        #expect(!WaveSongQuery("cloud").matches(song(state: .bookmarkOnly)))
        #expect(WaveSongQuery("offline").matches(song(state: .bookmarkOnly), availability: .onDevice))
        #expect(WaveSongQuery("cloud").matches(song(state: .bookmarkOnly), availability: .inCloud))
        #expect(!WaveSongQuery("offline").matches(song(), availability: .unknown))
        #expect(WaveSongQuery("ARTIST weather").matches(song()))
        #expect(!WaveSongQuery("night year:2020").matches(song()))
        #expect(!WaveSongQuery("favorites").matches(song(favorite: 0)))
        #expect(WaveSongQuery("   ").matches(song()))
    }
    @Test func tableSortsNumericMetadataAndLeavesUnknownValuesLast() {
        func track(_ id: Int64, duration: Double?, size: Int64?) -> Song {
            Song(id: id, songKey: id, artist: "Artist", title: "Track \(id)", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: id, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly, duration: duration, fileSize: size)
        }
        let long = track(1, duration: 100, size: 9)
        let short = track(2, duration: 9, size: 1000)
        let unknown = track(3, duration: nil, size: nil)
        let original = [unknown, long, short]
        #expect(waveSortedSongs(original, column: nil, ascending: true).map(\.uniqueId) == [3, 1, 2])
        #expect(waveSortedSongs(original, column: "Time", ascending: true).map(\.uniqueId) == [2, 1, 3])
        #expect(waveSortedSongs(original, column: "Time", ascending: false).map(\.uniqueId) == [1, 2, 3])
        #expect(waveSortedSongs(original, column: "Size", ascending: true).map(\.uniqueId) == [1, 2, 3])
        #expect(waveSortedSongs(original, column: "Size", ascending: false).map(\.uniqueId) == [2, 1, 3])
        #expect(waveDuration(125) == "2:05")
        #expect(waveDuration(nil) == "—")
        #expect(waveDuration(.infinity) == "—")
        #expect(waveDuration(.greatestFiniteMagnitude) == "—")
    }

    @Test func findObjectNamesAndHighlightsExcludeTrackOnlyWordsAndFilterTokens() {
        let trackOnly = WaveSongQuery("Night offline")
        #expect(trackOnly.hasFilters)
        #expect(!trackOnly.matchesWords(in: "Weather Test Artist"))
        #expect(trackOnly.matchesWords(in: "Night Weather"))
        #expect(!WaveSongQuery("Test Artist").hasFilters)
        let marked = waveHighlightedText("Night [Air] offline", query: "[Air] offline")
        let highlighted = marked.runs.filter { $0.backgroundColor != nil }.map { String(marked[$0.range].characters) }
        #expect(highlighted == ["[Air]"])
    }

    @Test func cloudFormatFilterUsesSavedMetadataWithoutAFileBookmark() {
        let cloud = Song(id: 10, songKey: 10, artist: "Cloud Artist", title: "Cloud Track", album: "Cloud Album", albumArtist: "Cloud Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 10, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly, sourceId: 1, relativePath: "Cloud Track.flac", fileExtension: "flac")
        #expect(WaveSongQuery("cloud flac").matches(cloud, availability: .inCloud))
        #expect(!WaveSongQuery("cloud flac").matches(cloud, availability: .unknown))
    }

    @Test func automaticListsKeepOnlyTheNewestOrMostPlayedTwentyFour() {
        let songs = (1...30).map { number -> Song in
            let id = Int64(number)
            return Song(id: id, songKey: id, artist: "Artist", title: "Track \(id)", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: id, createdAt: Date(timeIntervalSince1970: Double(number)), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly)
        }
        let recentIDs: [Int64] = waveRecentlyAddedSongs(songs).map(\.uniqueId)
        let expectedRecent: [Int64] = (7...30).reversed().map { Int64($0) }
        #expect(recentIDs == expectedRecent)
        let counts = Dictionary(uniqueKeysWithValues: (1...26).map { (String($0), $0) })
        let playedIDs: [Int64] = waveMostPlayedSongs(songs, counts: counts).map(\.uniqueId)
        let expectedPlayed: [Int64] = (3...26).reversed().map { Int64($0) }
        #expect(playedIDs == expectedPlayed)
        #expect(waveMostPlayedSongs(songs, counts: [:]).isEmpty)
        #expect(waveRecentlyAddedSongs(Array(songs.prefix(3))).count == 3)
    }

    @Test func verifiedOfflineCopyRemainsPlayableWhenItsSourceNeedsReconnect() {
        let linked = Song(id: 42, songKey: 42, artist: "Artist", title: "Cached track", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 42, createdAt: Date(), updatedAt: nil, localFilePath: "Music/cached.flac", fileState: .copied, sourceId: 7, relativePath: "cached.flac")
        let failedSource = [Int64(7): "Drive"]
        #expect(waveBlockedSourceName(song: linked, availability: .onDevice, blockedSources: failedSource) == nil)
        #expect(waveBlockedSourceName(song: linked, availability: .inCloud, blockedSources: failedSource) == "Drive")
        #expect(waveBlockedSourceName(song: linked, availability: .unknown, blockedSources: failedSource) == "Drive")
        #expect(waveBlockedSourceName(song: linked, availability: .unknown, blockedSources: [:]) == nil)
    }

    @Test func findMembershipDoesNotReorderOrDuplicateTheLibrary() {
        func track(_ id: Int64) -> Song {
            Song(id: id, songKey: id, artist: "Artist", title: "Same title", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: id, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly)
        }
        let insertionOrder = [track(1), track(2), track(3)]
        #expect(waveFindSongsInOriginalOrder(insertionOrder, matchingIDs: [3, 1, 3, 99]).map(\.uniqueId) == [1, 3])
        #expect(waveFindSongsInOriginalOrder(insertionOrder, matchingIDs: []).isEmpty)
        #expect(waveFindSongsInOriginalOrder(insertionOrder, matchingIDs: nil).map(\.uniqueId) == [1, 2, 3])
        #expect(waveFindSongsInOriginalOrder(insertionOrder, matchingIDs: [3], matchingQuery: "previous", currentQuery: "next").map(\.uniqueId) == [1, 2, 3], "A new query must not reuse the previous query's FTS membership while its asynchronous search runs")
        #expect(waveFindSongsInOriginalOrder(insertionOrder, matchingIDs: [3], matchingQuery: "next", currentQuery: "next").map(\.uniqueId) == [3])
    }

}

private actor RemovalTestRepository: PlaylistSongRepository {
    enum Failure: Error { case unavailable }
    private let tracks: [Song]
    private var order: [Int64]
    private var failedID: Int64?

    init(_ tracks: [Song]) {
        self.tracks = tracks
        order = tracks.map(\.uniqueId)
    }
    func failRemoval(of id: Int64?) { failedID = id }
    func removeSong(playlistId: Int64, songId: Int64) async throws {
        if songId == failedID { throw Failure.unavailable }
        order.removeAll { $0 == songId }
    }
    func addSong(playlistId: Int64, songId: Int64) async throws { order.append(songId) }
    func getSongs(playlistId: Int64) async throws -> [Song] {
        order.compactMap { id in tracks.first { $0.uniqueId == id } }
    }
    func reorderSongs(playlistId: Int64, newOrder: [Int64]) async throws { order = newOrder }
}
