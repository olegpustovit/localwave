import Foundation
import LocalWaveDomain
import LocalWaveData
import SQLite
import Testing
@testable import localwave

struct TrackMenuTests {
    @MainActor @Test func playNextKeepsTheCurrentDuplicateEntryAndMovesOnlyNewEntries() throws {
        let database = try Connection(.inMemory)
        let player = PlayerViewModel(playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database))
        let first = track(81)
        let last = track(82)
        player.configureQueue(songs: [first, first, last], startIndex: 1)
        let originalIDs = player.queueEntryIdentifiers

        LWTrackMenu.queueNext([first, last], player: player)

        #expect(player.queue.map(\.uniqueId) == [81, 81, 81, 82, 82])
        #expect(player.currentQueueIndex == 1)
        #expect(player.queueEntryIdentifiers[0] == originalIDs[0])
        #expect(player.queueEntryIdentifiers[1] == originalIDs[1])
        #expect(player.queueEntryIdentifiers[4] == originalIDs[2])
        #expect(Set(player.queueEntryIdentifiers).count == 5)
        player.stop()
    }

    @MainActor @Test func playNextIntoAnEmptyQueueSeedsAPausedCurrentTrack() throws {
        let database = try Connection(.inMemory)
        let player = PlayerViewModel(playlistRepo: try SQLitePlaylistRepository(db: database), playlistSongRepo: try SQLitePlaylistSongRepository(db: database))
        LWTrackMenu.queueNext([track(91), track(92)], player: player)
        #expect(player.currentSong?.uniqueId == 91)
        #expect(player.currentQueueIndex == 0)
        #expect(player.queue.map(\.uniqueId) == [91, 92])
        #expect(!player.isPlaying)
        player.stop()
    }

    private func track(_ id: Int64) -> Song {
        Song(id: id, songKey: id, artist: "Artist", title: "Track \(id)", album: "Album", albumArtist: "Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: id, createdAt: Date(), updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly)
    }
}
