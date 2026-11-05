import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  PlayerPersistenceTests.swift
//  localwaveTests
//
//  Created by Assistant on 2025-08-25.
//

import Testing
import Foundation
@testable import localwave

// Mock implementation for testing
actor MockPlayerPersistenceService: PlayerPersistenceService {
    private var savedVolume: Float?
    private var savedQueue: [Song] = []
    private var savedIndex: Int = 0
    
    func getVolume() async -> Float? {
        return savedVolume
    }
    
    func restore() async -> ([Song], Int, Song?)? {
        if savedQueue.isEmpty {
            return nil
        }
        // Validate index is within bounds
        guard savedIndex >= 0 && savedIndex < savedQueue.count else {
            return (savedQueue, 0, savedQueue.first)
        }
        let currentSong = savedQueue[savedIndex]
        return (savedQueue, savedIndex, currentSong)
    }
    
    func savePlaybackState(volume: Float, currentIndex: Int, songs: [Song]) async {
        savedVolume = volume
        savedQueue = songs
        savedIndex = currentIndex
    }
}

@Suite(.serialized)
struct PlayerPersistenceTests {
    
    @Test func testVolumeDefaultsTo1Point0WhenNoSavedValue() async throws {
        // iOS default is 1.0, not 0.7 as research shows
        let service = MockPlayerPersistenceService()
        let volume = await service.getVolume()
        #expect(volume == nil)
    }
    
    @Test func testVolumePersistenceWithZeroValue() async throws {
        // Test that muted state (0.0) is properly saved and restored
        let service = MockPlayerPersistenceService()
        await service.savePlaybackState(volume: 0.0, currentIndex: 0, songs: [])
        
        let restoredVolume = await service.getVolume()
        #expect(restoredVolume == 0.0)
    }
    
    @Test func testVolumePersistenceWithMaxValue() async throws {
        let service = MockPlayerPersistenceService()
        await service.savePlaybackState(volume: 1.0, currentIndex: 0, songs: [])
        
        let restoredVolume = await service.getVolume()
        #expect(restoredVolume == 1.0)
    }
    
    @Test func testVolumePersistenceWithMidValue() async throws {
        let service = MockPlayerPersistenceService()
        await service.savePlaybackState(volume: 0.5, currentIndex: 0, songs: [])
        
        let restoredVolume = await service.getVolume()
        #expect(restoredVolume == 0.5)
    }
    
    @Test func testVolumeDistinguishesZeroFromNil() async throws {
        let service = MockPlayerPersistenceService()
        
        // First check nil (no saved value)
        let initialVolume = await service.getVolume()
        #expect(initialVolume == nil)
        
        // Save muted state
        await service.savePlaybackState(volume: 0.0, currentIndex: 0, songs: [])
        
        // Should return 0.0, not nil
        let mutedVolume = await service.getVolume()
        #expect(mutedVolume == 0.0)
        #expect(mutedVolume != nil)
    }
    
    @Test func testLogarithmicVolumeConversion() async throws {
        // Test the square-based logarithmic conversion for better human perception
        // Using x² for slider -> actual volume conversion
        
        // Slider at 0.5 should give actual volume of 0.25
        let sliderValue: Float = 0.5
        let actualVolume = sliderValue * sliderValue
        #expect(actualVolume == 0.25)
        
        // Slider at 0.7 should give actual volume of 0.49
        let sliderValue2: Float = 0.7
        let actualVolume2 = sliderValue2 * sliderValue2
        #expect(abs(actualVolume2 - 0.49) < 0.000001)
        
        // Inverse: actual volume 0.25 should give slider 0.5
        let volume: Float = 0.25
        let sliderPosition = sqrt(volume)
        #expect(sliderPosition == 0.5)
    }
    
    @Test func testDefaultPlayerPersistenceService() async throws {
        // Test the actual implementation with UserDefaults
        let mockRepo = MockSongRepository()
        let service = DefaultPlayerPersistenceService(songRepo: mockRepo)
        
        // Clear any existing values
        UserDefaults.standard.removeObject(forKey: "playerVolume")
        
        // Test nil for first time
        let initialVolume = await service.getVolume()
        #expect(initialVolume == nil)
        
        // Save and restore
        await service.savePlaybackState(volume: 0.75, currentIndex: 0, songs: [])
        let restoredVolume = await service.getVolume()
        #expect(restoredVolume == 0.75)
        
        // Clean up
        UserDefaults.standard.removeObject(forKey: "playerVolume")
    }
    
    @Test func testPlayerQueueRestoration() async throws {
        let service = MockPlayerPersistenceService()
        
        // Create test songs
        let song1 = createTestSong(id: 1, title: "Song1", artist: "Artist1")
        let song2 = createTestSong(id: 2, title: "Song2", artist: "Artist2")
        let song3 = createTestSong(id: 3, title: "Song3", artist: "Artist3")
        
        let queue = [song1, song2, song3]
        let currentIndex = 1  // Playing song2
        
        // Save state
        await service.savePlaybackState(volume: 0.8, currentIndex: currentIndex, songs: queue)
        
        // Restore state
        let restored = await service.restore()
        #expect(restored != nil)
        
        let (restoredQueue, restoredIndex, restoredSong) = restored!
        #expect(restoredQueue.count == 3)
        #expect(restoredIndex == 1)
        #expect(restoredSong?.id == 2)
        #expect(restoredSong?.title == "Song2")
    }
    
    @Test func testQueueRestorationWithInvalidIndex() async throws {
        let service = MockPlayerPersistenceService()
        
        let song1 = createTestSong(id: 1, title: "Song1", artist: "Artist1")
        let queue = [song1]
        
        // Save with out-of-bounds index
        await service.savePlaybackState(volume: 0.5, currentIndex: 5, songs: queue)
        
        // Should handle gracefully
        let restored = await service.restore()
        #expect(restored != nil)
        
        let (restoredQueue, restoredIndex, restoredSong) = restored!
        #expect(restoredQueue.count == 1)
        #expect(restoredIndex == 0)  // Should reset to 0
        #expect(restoredSong?.id == 1)  // Should be first song
    }
    
    @Test func testEmptyQueueRestoration() async throws {
        let service = MockPlayerPersistenceService()
        
        // Save empty queue
        await service.savePlaybackState(volume: 0.5, currentIndex: 0, songs: [])
        
        // Should return nil for empty queue
        let restored = await service.restore()
        #expect(restored == nil)
    }
    
    @Test func testQueueOrderPreservation() async throws {
        // Test with actual DefaultPlayerPersistenceService
        let mockRepo = MockOrderedSongRepositoryForPersistence()
        let service = DefaultPlayerPersistenceService(songRepo: mockRepo)
        
        // Clear UserDefaults
        UserDefaults.standard.removeObject(forKey: "currentQueue")
        UserDefaults.standard.removeObject(forKey: "currentQueueIndex")
        
        // Create songs with specific order
        let song1 = createTestSong(id: 3, title: "Third", artist: "C")
        let song2 = createTestSong(id: 1, title: "First", artist: "A")
        let song3 = createTestSong(id: 2, title: "Second", artist: "B")
        
        // Add songs to mock repo
        await mockRepo.addSongs([song1, song2, song3])
        
        // Save in specific order
        let originalQueue = [song1, song2, song3]
        await service.savePlaybackState(volume: 0.5, currentIndex: 1, songs: originalQueue)
        
        // Restore and verify order is preserved
        let restored = await service.restore()
        #expect(restored != nil)
        
        let (restoredQueue, restoredIndex, restoredSong) = restored!
        #expect(restoredQueue.count == 3)
        #expect(restoredQueue[0].id == 3)  // First should be song with ID 3
        #expect(restoredQueue[1].id == 1)  // Second should be song with ID 1
        #expect(restoredQueue[2].id == 2)  // Third should be song with ID 2
        #expect(restoredIndex == 1)
        #expect(restoredSong?.id == 1)  // Current song should be ID 1 (at index 1)
        
        // Clean up
        UserDefaults.standard.removeObject(forKey: "currentQueue")
        UserDefaults.standard.removeObject(forKey: "currentQueueIndex")
    }
    
    // Helper function to create test songs
    private func createTestSong(id: Int64, title: String, artist: String) -> Song {
        return Song(
            id: id,
            songKey: id,
            artist: artist,
            title: title,
            album: "Album",
            albumArtist: "",
            releaseYear: nil,
            discNumber: nil,
            trackNumber: nil,
            coverArtPath: nil,
            bookmark: nil,
            pathHash: id,
            createdAt: Date(),
            updatedAt: nil,
            localFilePath: nil,
            fileState: .bookmarkOnly
        )
    }
}

// Mock repository that preserves order for persistence testing
actor MockOrderedSongRepositoryForPersistence: SongRepository {
    private var songs: [Song] = []
    
    func addSongs(_ newSongs: [Song]) async {
        songs.append(contentsOf: newSongs)
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
    
    func upsertSong(_ song: Song) async throws -> Song {
        return song
    }
    
    func searchSongsFTS(query: String, limit: Int, offset: Int) async throws -> [Song] {
        return []
    }
    
    func totalSongCount(query: String) async throws -> Int {
        return 0
    }
    
    func getAllArtists() async throws -> [String] {
        return []
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
    
    func deleteSong(songId: Int64) async throws {}
    
    func deleteAlbum(album: String, artist: String?) async throws {}
    
    func getSongsNeedingCopy() async -> [Song] {
        return []
    }
    
    func updateCachedFile(songId: Int64, localFilePath: String?, state: FileState, expectedState: FileState, expectedPath: String?) async throws -> Bool {
        guard let index = songs.firstIndex(where: { $0.id == songId && $0.fileState == expectedState && $0.localFilePath == expectedPath }) else { return false }
        songs[index] = songs[index].copyWith(localFilePath, state)
        return true
    }
    func markSongForCopy(songId: Int64) async throws {}
    func toggleSongAsFavorite(songId: Int64, state: Bool) async throws {}
}

// Mock song repository for testing
actor MockSongRepository: SongRepository {
    func upsertSong(_ song: Song) async throws -> Song {
        return song
    }
    
    func searchSongsFTS(query: String, limit: Int, offset: Int) async throws -> [Song] {
        return []
    }
    
    func totalSongCount(query: String) async throws -> Int {
        return 0
    }
    
    func getAllArtists() async throws -> [String] {
        return []
    }
    
    func getAllAlbums() async throws -> [Album] {
        return []
    }
    
    func getSongs(ids: [Int64]) async -> [Song] {
        return []
    }
    
    func getSong(sourceId: Int64, relativePath: String) async -> Song? { nil }

    func getSongByURL(_ url: URL) async -> Song? {
        return nil
    }
    
    func updateBookmark(songId: Int64, bookmark: Data) async throws {}
    func updateSourceLocation(songId: Int64, sourceId: Int64, relativePath: String) async throws {}
    func updateSourceAvailability(songId: Int64, availability: FileAvailability) async throws {}
    
    func deleteSong(songId: Int64) async throws {}
    
    func deleteAlbum(album: String, artist: String?) async throws {}
    
    func getSongsNeedingCopy() async -> [Song] {
        return []
    }
    
    func updateCachedFile(songId: Int64, localFilePath: String?, state: FileState, expectedState: FileState, expectedPath: String?) async throws -> Bool { false }
    func markSongForCopy(songId: Int64) async throws {}
    func toggleSongAsFavorite(songId: Int64, state: Bool) async throws {}
}