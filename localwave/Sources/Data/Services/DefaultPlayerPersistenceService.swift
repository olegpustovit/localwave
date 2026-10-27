import Foundation
import LocalWaveDomain
import LocalWaveCore
import os

public actor DefaultPlayerPersistenceService: PlayerPersistenceService {
    private let queueKey = "currentQueue"
    private let currentIndexKey = "currentQueueIndex"
    private let volumeKey = "playerVolume"

    private let songRepo: SongRepository

    public init(songRepo: SongRepository) {
        self.songRepo = songRepo
    }

    public func getVolume() async -> Float? {
        // Check if volume has been explicitly set
        if UserDefaults.standard.object(forKey: volumeKey) != nil {
            return UserDefaults.standard.float(forKey: volumeKey)
        }
        // Return nil to indicate no saved volume
        return nil
    }

    let logger = createLogger(subsystem: subsystem, category: "PlayerPersistenceService")

    public func restore() async -> ([Song], Int, Song?)? {
        guard let songIds = UserDefaults.standard.array(forKey: queueKey) as? [Int64],
              let currentIndex = UserDefaults.standard.value(forKey: currentIndexKey) as? Int,
              !songIds.isEmpty
        else {
            logger.debug("no persisted data, skipping")
            return nil
        }

        logger.debug("Restoring player state - Queue size: \(songIds.count), Current index: \(currentIndex)")
        logger.debug("Song IDs in order: \(songIds)")
        
        let songs = await songRepo.getSongs(ids: songIds)
        
        logger.debug("Retrieved \(songs.count) songs from repository")
        
        // Validate index is within bounds
        guard currentIndex >= 0 && currentIndex < songs.count else {
            logger.error("Invalid current index \(currentIndex) for \(songs.count) songs")
            return (songs, 0, songs.first)
        }
        
        let currentSong = songs[currentIndex]
        logger.debug("Current song at index \(currentIndex): \(currentSong.title) by \(currentSong.artist)")

        return (songs, currentIndex, currentSong)
    }

    public func savePlaybackState(volume: Float, currentIndex: Int, songs: [Song]) async {
        let songIds = songs.map { $0.id ?? -1 }
        
        logger.debug("Saving player state - Queue size: \(songs.count), Current index: \(currentIndex), Volume: \(volume)")
        if currentIndex >= 0 && currentIndex < songs.count {
            let currentSong = songs[currentIndex]
            logger.debug("Current song being saved: \(currentSong.title) by \(currentSong.artist)")
        }
        
        UserDefaults.standard.set(songIds, forKey: queueKey)
        UserDefaults.standard.set(currentIndex, forKey: currentIndexKey)
        UserDefaults.standard.set(volume, forKey: volumeKey)
    }
}
