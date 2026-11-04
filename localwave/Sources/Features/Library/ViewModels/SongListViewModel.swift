//
//  SongListViewModel.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

@MainActor
class SongListViewModel: ObservableObject {
    enum Filter {
        case all
        case artist(String)
        case album(String, artist: String?)
    }

    private let filter: Filter
    private let songRepo: SongRepository
    @Published var songs: [Song] = []
    @Published var totalSongs: Int = 0
    @Published var isLoadingPage: Bool = false

    private var currentPage: Int = 0
    private let pageSize: Int = 50
    private var hasMorePages: Bool = true
    private var currentQuery: String = ""

    private let logger = Logger(
        subsystem: "com.snowbear.localwave",
        category: "SongListViewModel"
    )

    init(songRepo: SongRepository, filter: Filter) {
        self.songRepo = songRepo
        self.filter = filter
    }

    func reset() {
        currentPage = 0
        songs = []
        hasMorePages = true
    }

    func toggleFavorite(_ id: Int64?, state: Bool) async throws {
        if ((id) != nil) {
            try await songRepo.toggleSongAsFavorite(songId: id!, state: state)
        }
    }
        
    private func loadFilteredSongs() async throws -> [Song] {
        switch filter {
        case .all:
            return try await songRepo.searchSongsFTS(
                query: currentQuery,
                limit: pageSize,
                offset: currentPage * pageSize
            )

        case let .artist(artist):
            let artistFilter = "artist:\"\(artist)\""
            let combinedQuery =
                currentQuery.isEmpty ? artistFilter : "\(currentQuery) \(artistFilter)"
            return try await songRepo.searchSongsFTS(
                query: combinedQuery,
                limit: pageSize,
                offset: currentPage * pageSize
            )

        case let .album(album, _):
            let albumFilter = "album:\"\(album)\""
            let artistFilter = ""

            let combinedQuery: String
            if currentQuery.isEmpty {
                combinedQuery = [albumFilter, artistFilter].filter { !$0.isEmpty }.joined(
                    separator: " ")
            } else {
                combinedQuery = "\(currentQuery) \(albumFilter) \(artistFilter)"
            }

            let cleanedQuery =
                combinedQuery
                    .components(separatedBy: .whitespaces)
                    .filter { !$0.isEmpty }
                    .joined(separator: " ")

            logger.debug("Album query: \(cleanedQuery)")

            var results = try await songRepo.searchSongsFTS(
                query: cleanedQuery,
                limit: pageSize,
                offset: currentPage * pageSize
            )

            if !results.isEmpty {
                results.sort {
                    if $0.trackNumber == $1.trackNumber {
                        return $0.artist.count < $1.artist.count
                    }
                    return ($0.trackNumber ?? Int.max) < ($1.trackNumber ?? Int.max)
                }
            }
            logger.debug("Returning \(results.count) songs for album \(album)")
            return results
        }
    }

    private func loadTotalSongs() async {
        do {
            let count = try await songRepo.totalSongCount(query: currentQuery)
            totalSongs = count
            logger.debug("Total songs loaded: \(count)")
        } catch {
            logger.error("Failed to load total song count: \(error.localizedDescription)")
            totalSongs = 0
        }
    }

    func loadMoreIfNeeded(currentSong song: Song) {
        if let index = songs.firstIndex(where: { $0.id == song.id }), index >= songs.count - 5 {
            Task { await loadMoreSongs() }
        }
    }

    func loadMoreSongs() async {
        guard !isLoadingPage && hasMorePages else { return }
        isLoadingPage = true
        defer { isLoadingPage = false }

        do {
            let newSongs = try await loadFilteredSongs()
            let received = newSongs.count

            hasMorePages = received >= pageSize

            if case .album = filter {
                logger.debug("need to load \(newSongs.count) songs and sort them")
                songs.append(contentsOf: newSongs)
                sortSongs()
            } else {
                songs.append(contentsOf: newSongs)
            }

            if received > 0 {
                currentPage += 1
            }
        } catch {
            logger.error("Error loading more songs: \(error.localizedDescription)")
            hasMorePages = false
        }
    }

    func sortSongs() {
        songs.sort { s1, s2 in
            if let t1 = s1.trackNumber, let t2 = s2.trackNumber {
                return t1 < t2
            }
            return s1.title.localizedStandardCompare(s2.title) == .orderedAscending
        }
    }
    
    func searchSongs(query: String) async {
        currentQuery = query
        reset()
        await loadTotalSongs()

        do {
            let newSongs = try await loadFilteredSongs()
            if case .album = filter {
                songs = newSongs.sorted { s1, s2 in
                    if let t1 = s1.trackNumber, let t2 = s2.trackNumber {
                        return t1 < t2
                    }
                    return s1.title.localizedStandardCompare(s2.title) == .orderedAscending
                }
            } else {
                songs = newSongs
            }
            if newSongs.count < pageSize {
                hasMorePages = false
            } else {
                currentPage = 1
            }
            hasMorePages = newSongs.count >= pageSize
        } catch {
            logger.error("Search error: \(error.localizedDescription)")
        }
    }

    func loadInitialSongs() async {
        reset()
        do {
            let initialSongs = try await loadFilteredSongs()
            songs = initialSongs // Overwrite the array
            if case .album = filter {
                songs = initialSongs.sorted { s1, s2 in
                    if let t1 = s1.trackNumber, let t2 = s2.trackNumber {
                        return t1 < t2
                    }
                    return s1.title.localizedStandardCompare(s2.title) == .orderedAscending
                }
            } else {
                songs = initialSongs
            }
            if initialSongs.count == pageSize {
                hasMorePages = true
                currentPage = 1
            }
            hasMorePages = initialSongs.count >= pageSize
            await loadTotalSongs()
        } catch {
            logger.error("Initial load error: \(error.localizedDescription)")
            hasMorePages = false
        }
    }
}
