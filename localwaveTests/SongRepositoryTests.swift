import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  SongRepositoryTests.swift
//  localwaveTests
//
//  Created by Assistant on 2025-08-25.
//

import Testing
import Foundation
@testable import localwave

// Mock implementation for testing
actor MockOrderedSongRepository: SongRepository {
    private(set) var copyRequests: [Int64] = []
    private var songs: [Song] = []
    private var nextId: Int64 = 1
    
    init(songs: [Song] = []) {
        self.songs = songs
    }
    
    func upsertSong(_ song: Song) async throws -> Song {
        var updatedSong = song
        if updatedSong.id == nil {
            updatedSong = updatedSong.copyWith(id: nextId)
            nextId += 1
            songs.append(updatedSong)
        } else {
            if let index = songs.firstIndex(where: { $0.id == song.id }) {
                songs[index] = updatedSong
            }
        }
        return updatedSong
    }
    
    func getSongs(ids: [Int64]) async -> [Song] {
        // Create a dictionary for O(1) lookup
        let songDict: [Int64: Song] = Dictionary(uniqueKeysWithValues: songs.compactMap { song in
            guard let id = song.id else { return nil }
            return (id, song)
        })
        
        // Return songs in the same order as the input IDs
        return ids.compactMap { songDict[$0] }
    }
    
    func searchSongsFTS(query: String, limit: Int, offset: Int) async throws -> [Song] {
        return []
    }
    
    func totalSongCount(query: String) async throws -> Int {
        return songs.count
    }
    
    func getAllArtists() async throws -> [String] {
        return Array(Set(songs.map { $0.artist }))
    }
    
    func getAllAlbums() async throws -> [Album] {
        return []
    }
    
    func getSong(sourceId: Int64, relativePath: String) async -> Song? { nil }

    func getSongByURL(_ url: URL) async -> Song? {
        return nil
    }
    
    func updateBookmark(songId: Int64, bookmark: Data) async throws {}
    func updateSourceLocation(songId: Int64, sourceId: Int64, relativePath: String) async throws {}
    func updateSourceAvailability(songId: Int64, availability: FileAvailability) async throws {}
    
    func deleteSong(songId: Int64) async throws {
        songs.removeAll { $0.id == songId }
    }
    
    func deleteAlbum(album: String, artist: String?) async throws {}
    
    func getSongsNeedingCopy() async -> [Song] {
        return []
    }
    
    func updateCachedFile(songId: Int64, localFilePath: String?, state: FileState, expectedState: FileState, expectedPath: String?) async throws -> Bool {
        guard let index = songs.firstIndex(where: { $0.id == songId && $0.fileState == expectedState && $0.localFilePath == expectedPath }) else { return false }
        songs[index] = songs[index].copyWith(localFilePath, state)
        return true
    }
    func markSongForCopy(songId: Int64) async throws { copyRequests.append(songId) }
    func toggleSongAsFavorite(songId: Int64, state: Bool) async throws {}
}

struct SongRepositoryTests {
    
    @Test func testGetSongsPreservesInputOrder() async throws {
        // Create test songs with specific IDs
        let song1 = Song(
            id: 1, songKey: 1, artist: "Artist1", title: "Title1",
            album: "Album1", albumArtist: "", releaseYear: nil,
            discNumber: nil, trackNumber: nil, coverArtPath: nil,
            bookmark: nil, pathHash: 1, createdAt: Date(),
            updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly
        )
        
        let song2 = Song(
            id: 2, songKey: 2, artist: "Artist2", title: "Title2",
            album: "Album2", albumArtist: "", releaseYear: nil,
            discNumber: nil, trackNumber: nil, coverArtPath: nil,
            bookmark: nil, pathHash: 2, createdAt: Date(),
            updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly
        )
        
        let song3 = Song(
            id: 3, songKey: 3, artist: "Artist3", title: "Title3",
            album: "Album3", albumArtist: "", releaseYear: nil,
            discNumber: nil, trackNumber: nil, coverArtPath: nil,
            bookmark: nil, pathHash: 3, createdAt: Date(),
            updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly
        )
        
        let repo = MockOrderedSongRepository(songs: [song1, song2, song3])
        
        // Request songs in different order: 3, 1, 2
        let requestedIds: [Int64] = [3, 1, 2]
        let returnedSongs = await repo.getSongs(ids: requestedIds)
        
        // Verify order is preserved
        #expect(returnedSongs.count == 3)
        #expect(returnedSongs[0].id == 3)
        #expect(returnedSongs[1].id == 1)
        #expect(returnedSongs[2].id == 2)
    }
    
    @Test func testGetSongsHandlesMissingIds() async throws {
        let song1 = Song(
            id: 1, songKey: 1, artist: "Artist1", title: "Title1",
            album: "Album1", albumArtist: "", releaseYear: nil,
            discNumber: nil, trackNumber: nil, coverArtPath: nil,
            bookmark: nil, pathHash: 1, createdAt: Date(),
            updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly
        )
        
        let repo = MockOrderedSongRepository(songs: [song1])
        
        // Request songs including non-existent IDs
        let requestedIds: [Int64] = [1, 999, 2]
        let returnedSongs = await repo.getSongs(ids: requestedIds)
        
        // Should only return the existing song
        #expect(returnedSongs.count == 1)
        #expect(returnedSongs[0].id == 1)
    }
    
    @Test func testGetSongsHandlesEmptyInput() async throws {
        let song1 = Song(
            id: 1, songKey: 1, artist: "Artist1", title: "Title1",
            album: "Album1", albumArtist: "", releaseYear: nil,
            discNumber: nil, trackNumber: nil, coverArtPath: nil,
            bookmark: nil, pathHash: 1, createdAt: Date(),
            updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly
        )
        
        let repo = MockOrderedSongRepository(songs: [song1])
        
        // Request empty array
        let requestedIds: [Int64] = []
        let returnedSongs = await repo.getSongs(ids: requestedIds)
        
        // Should return empty array
        #expect(returnedSongs.count == 0)
    }
    
    @Test func testGetSongsHandlesDuplicateIds() async throws {
        let song1 = Song(
            id: 1, songKey: 1, artist: "Artist1", title: "Title1",
            album: "Album1", albumArtist: "", releaseYear: nil,
            discNumber: nil, trackNumber: nil, coverArtPath: nil,
            bookmark: nil, pathHash: 1, createdAt: Date(),
            updatedAt: nil, localFilePath: nil, fileState: .bookmarkOnly
        )
        
        let repo = MockOrderedSongRepository(songs: [song1])
        
        // Request same ID multiple times
        let requestedIds: [Int64] = [1, 1, 1]
        let returnedSongs = await repo.getSongs(ids: requestedIds)
        
        // Should return the song multiple times in order
        #expect(returnedSongs.count == 3)
        #expect(returnedSongs[0].id == 1)
        #expect(returnedSongs[1].id == 1)
        #expect(returnedSongs[2].id == 1)
    }
}
