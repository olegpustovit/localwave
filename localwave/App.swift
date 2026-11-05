#if os(iOS)
import LocalWaveDomain
import LocalWaveCore
import LocalWaveData
import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

@main
struct LocalWave: App {
    @StateObject private var dependencies: DependencyContainer
    @StateObject private var playerVM: PlayerViewModel
    @StateObject private var tabState = TabState()

    init() {
        #if DEBUG
        if LWReferenceFixture.enabled { UIScrollView.appearance().contentInsetAdjustmentBehavior = .never }
        if let testID = ProcessInfo.processInfo.environment["LOCALWAVE_UI_TEST_ID"], UUID(uuidString: testID) != nil {
            let onboarding = (LWReferenceFixture.enabled && LWReferenceFixture.screen == "onboarding")
                || ProcessInfo.processInfo.environment["LOCALWAVE_UI_EMPTY_LIBRARY"] == "1"
            LocalWavePreferences.current.set(!onboarding, forKey: "lwDidStart")
        }
        #endif
        let c = try! DependencyContainer()
        _dependencies = StateObject(wrappedValue: c)

        _playerVM = StateObject(
            wrappedValue: PlayerViewModel(
                playerPersistenceService: c.playerPersistenceService,
                songRepo: c.songRepository,
                playlistRepo: c.playlistRepo,
                playlistSongRepo: c.playlistSongRepo,
                sourceAccessResolver: { song in try await c.resolveSourceAccess(for: song) }
            ))
    }

    var body: some Scene {
        WindowGroup {
            MainTabView(dependencies: dependencies)
                .environmentObject(tabState)
                .environmentObject(dependencies)
                .environmentObject(playerVM)
                .applyTheme()
                .defaultAppStorage(LocalWavePreferences.current)
                #if DEBUG
                .task {
                    guard let testID = ProcessInfo.processInfo.environment["LOCALWAVE_UI_TEST_ID"], UUID(uuidString: testID) != nil else { return }
                    if LWReferenceFixture.enabled {
                        do { try await LWReferenceFixture.seed(dependencies, player: playerVM) }
                        catch { FileHandle.standardOutput.write(Data(("LOCALWAVE_REFERENCE_ERROR=\(error)\n").utf8)) }
                        return
                    }
                    if ProcessInfo.processInfo.environment["LOCALWAVE_UI_EMPTY_LIBRARY"] == "1" { return }
                    let audioURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("ui-test-\(testID).caf")
                    if !FileManager.default.fileExists(atPath: audioURL.path),
                       let format = AVAudioFormat(standardFormatWithSampleRate: 48000, channels: 1),
                       let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 48000) {
                        buffer.frameLength = 48000
                        if let channel = buffer.floatChannelData?[0] {
                            let phaseStep: Double = 2.0 * Double.pi * 220.0 / 48000.0
                            for frame in 0..<48000 { channel[frame] = Float(sin(Double(frame) * phaseStep)) * Float(0.02) }
                            if let file = try? AVAudioFile(forWriting: audioURL, settings: format.settings) {
                                for _ in 0..<30 { try? file.write(from: buffer) }
                            }
                        }
                    }
                    let fixtureStartedAt = Date()
                    let fixtureCount = max(1, min(5000, Int(ProcessInfo.processInfo.environment["LOCALWAVE_UI_LIBRARY_COUNT"] ?? "1") ?? 1))
                    let fixtureSize = (try? audioURL.resourceValues(forKeys: [.fileSizeKey]))?.fileSize.map(Int64.init)
                    let song = Song(id: nil, songKey: 100, artist: "Test Artist", title: "Playlist Test Track", album: "Test Album", albumArtist: "Test Artist", releaseYear: nil, discNumber: nil, trackNumber: nil, coverArtPath: nil, bookmark: nil, pathHash: 100, createdAt: Date(), updatedAt: nil, localFilePath: audioURL.path, fileState: .copied, duration: 30, fileSize: fixtureSize, fileExtension: "caf", sampleRate: 48000, bitDepth: 32)
                    let storedSong = try? await dependencies.songRepository.upsertSong(song)
                    if fixtureCount > 1 {
                        do {
                            for index in 1..<fixtureCount {
                                try Task.checkCancellation()
                                let albumNumber = (index - 1) / 25
                                let artist = String(format: "Scale Artist %02d", albumNumber % 50)
                                let fixture = Song(id: nil, songKey: Int64(100 + index), artist: artist, title: String(format: "Scale Track %05d", index), album: String(format: "Scale Album %03d", albumNumber), albumArtist: artist, releaseYear: 2000 + albumNumber % 25, discNumber: 1, trackNumber: (index - 1) % 25 + 1, coverArtPath: nil, bookmark: nil, pathHash: Int64(100 + index), createdAt: Date(), updatedAt: nil, localFilePath: audioURL.path, fileState: .copied, isFavorite: index % 10 == 0 ? 1 : 0, duration: 30, fileSize: fixtureSize, fileExtension: "caf", sampleRate: 48000, bitDepth: 32)
                                _ = try await dependencies.songRepository.upsertSong(fixture)
                            }
                        } catch {
                            FileHandle.standardOutput.write(Data(("LOCALWAVE_FIXTURE_ERROR=\(error.localizedDescription)\n").utf8))
                        }
                    }
                    if ProcessInfo.processInfo.environment["LOCALWAVE_UI_SOURCE"] == "1" {
                        do {
                            let sourceDirectory = audioURL.deletingLastPathComponent().appendingPathComponent("ui-source-" + testID, isDirectory: true).appendingPathComponent("UI Test Source", isDirectory: true)
                            try FileManager.default.createDirectory(at: sourceDirectory, withIntermediateDirectories: true)
                            let wavURL = sourceDirectory.appendingPathComponent("Folder Test Track.wav")
                            if !FileManager.default.fileExists(atPath: wavURL.path) {
                                let reader = try AVAudioFile(forReading: audioURL)
                                let settings: [String: Any] = [AVFormatIDKey: kAudioFormatLinearPCM, AVSampleRateKey: 48000, AVNumberOfChannelsKey: 1, AVLinearPCMBitDepthKey: 16, AVLinearPCMIsFloatKey: false, AVLinearPCMIsBigEndianKey: false]
                                let writer = try AVAudioFile(forWriting: wavURL, settings: settings)
                                if let samples = AVAudioPCMBuffer(pcmFormat: reader.processingFormat, frameCapacity: 4096) {
                                    while reader.framePosition < reader.length {
                                        try reader.read(into: samples)
                                        try writer.write(from: samples)
                                    }
                                }
                            }
                            _ = try await dependencies.addMusicURLs([sourceDirectory])
                            FileHandle.standardOutput.write(Data("LOCALWAVE_UI_SOURCE_READY=1\n".utf8))
                        } catch {
                            FileHandle.standardOutput.write(Data(("LOCALWAVE_UI_SOURCE_ERROR=\(error.localizedDescription)\n").utf8))
                        }
                    }
                    let savedCount = (try? await dependencies.songRepository.totalSongCount(query: "")) ?? 0
                    FileHandle.standardOutput.write(Data(("LOCALWAVE_FIXTURE_READY=\(savedCount) SECONDS=\(Date().timeIntervalSince(fixtureStartedAt))\n").utf8))
                    _ = try? await dependencies.playlistRepo.create(playlist: Playlist(id: nil, name: "UI Test Playlist", createdAt: Date(), updatedAt: nil))
                    NotificationCenter.default.post(name: Notification.Name("SongListRefresh"), object: nil)
                    NotificationCenter.default.post(name: Notification.Name("LibraryRefresh"), object: nil)
                    if ProcessInfo.processInfo.environment["LOCALWAVE_UI_AUTOPLAY"] == "1", let storedSong {
                        let mute = playerVM.$playbackURL.sink { _ in playerVM.playbackEngine?.volume = 0 }
                        defer { mute.cancel() }
                        playerVM.configureQueue(songs: [storedSong], startIndex: 0, context: .album(name: storedSong.album, artist: storedSong.artist))
                        playerVM.playQueueEntry(at: 0)
                        for _ in 0..<500 {
                            if playerVM.isPlaying { break }
                            try? await Task.sleep(for: .milliseconds(10))
                        }
                        let before = playerVM.playbackEngine?.currentTime().seconds ?? 0
                        try? await Task.sleep(for: .seconds(2))
                        let after = playerVM.playbackEngine?.currentTime().seconds ?? 0
                        FileHandle.standardOutput.write(Data(("LOCALWAVE_PLAYBACK_VERIFIED=\(playerVM.isPlaying && after > before + 1)\n").utf8))
                    }
                }
                #endif
        }
    }
}

#endif
