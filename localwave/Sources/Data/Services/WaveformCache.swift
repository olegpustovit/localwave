import AVFoundation
import CryptoKit
import Foundation
import LocalWaveDomain
import SQLite

/// Audio-derived peaks survive view changes and launches; no provider download starts here.
public actor WaveformCache {
    public static let shared = WaveformCache()
    public static let peakCount = 512
    private let directory: URL
    private let audioCacheDirectory: URL
    private var database: Connection?
    private var extractions: [String: Task<[Float], Error>] = [:]

    public init(directory: URL? = nil, audioCacheDirectory: URL? = nil) {
        self.audioCacheDirectory = (audioCacheDirectory ?? FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("Music", isDirectory: true)).resolvingSymlinksInPath().standardizedFileURL
        self.directory = directory ?? FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("LocalWave", isDirectory: true)
    }

    public func peaks(for song: Song, sourceURL: URL? = nil) async throws -> [Float] {
        let db = try connection()
        let cached = try cachedPeaks(songKey: song.songKey, database: db)
        guard let url = readableURL(song: song, explicit: sourceURL) else { return cached?.peaks ?? [] }
        let access = url.startAccessingSecurityScopedResource()
        defer { if access { url.stopAccessingSecurityScopedResource() } }
        let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
        guard attributes[.type] as? FileAttributeType == .typeRegular else { return [] }
        let name = url.deletingPathExtension().lastPathComponent
        let contentAddressed = url.deletingLastPathComponent().resolvingSymlinksInPath().standardizedFileURL == audioCacheDirectory && name.count == 64 && name.allSatisfy { $0.isHexDigit }
        // Playback touches the cache date for LRU; its content hash remains the version.
        let version = contentAddressed ? name : url.standardizedFileURL.path + "|" + String((attributes[.modificationDate] as? Date)?.timeIntervalSince1970 ?? 0)
        let identity = version + "|" + String(describing: attributes[.size])
        let signature = SHA256.hash(data: Data(identity.utf8)).map { String(format: "%02x", $0) }.joined()
        if cached?.signature == signature { return cached?.peaks ?? [] }

        let extraction: Task<[Float], Error>
        if let pending = extractions[signature] { extraction = pending }
        else {
            extraction = Task.detached(priority: .utility) { try await Self.decode(url: url) }
            extractions[signature] = extraction
        }
        do {
            // Finish an in-flight extraction even if one viewer disappears; other viewers share it.
            let peaks = try await extraction.value
            extractions[signature] = nil
            guard peaks.count == Self.peakCount else { return [] }
            let data = try JSONEncoder().encode(peaks)
            try db.run("INSERT INTO waveforms(song_key, source_signature, peaks) VALUES (?, ?, ?) ON CONFLICT(song_key) DO UPDATE SET source_signature=excluded.source_signature, peaks=excluded.peaks", song.songKey, signature, Blob(bytes: Array(data)))
            return peaks
        } catch {
            extractions[signature] = nil
            throw error
        }
    }

    private func connection() throws -> Connection {
        if let database { return database }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let db = try Connection(directory.appendingPathComponent("waveforms.sqlite").path)
        try db.run("CREATE TABLE IF NOT EXISTS waveforms (song_key INTEGER PRIMARY KEY, source_signature TEXT NOT NULL, peaks BLOB NOT NULL)")
        database = db
        return db
    }

    private func cachedPeaks(songKey: Int64, database: Connection) throws -> (signature: String, peaks: [Float])? {
        for row in try database.prepare("SELECT source_signature, peaks FROM waveforms WHERE song_key = ?", songKey) {
            guard let signature = row[0] as? String, let blob = row[1] as? Blob,
                  blob.bytes.count <= 64 * 1024,
                  let peaks = try? JSONDecoder().decode([Float].self, from: Data(blob.bytes)),
                  peaks.count == Self.peakCount, peaks.allSatisfy({ $0.isFinite && (0...1).contains($0) }) else {
                try database.run("DELETE FROM waveforms WHERE song_key = ?", songKey)
                return nil
            }
            return (signature, peaks)
        }
        return nil
    }

    private func readableURL(song: Song, explicit: URL?) -> URL? {
        if let explicit, explicit.isFileURL, FileManager.default.isReadableFile(atPath: explicit.path) { return explicit }
        guard song.fileState == .copied, let path = song.localFilePath else { return nil }
        let url = path.hasPrefix("/") ? URL(fileURLWithPath: path) : FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent(path)
        return FileManager.default.isReadableFile(atPath: url.path) ? url : nil
    }

    public static func decode(url: URL) async throws -> [Float] {
        let asset = AVURLAsset(url: url)
        guard let track = try await asset.loadTracks(withMediaType: .audio).first else { return [] }
        let duration = try await asset.load(.duration).seconds
        guard duration.isFinite, duration > 0 else { return [] }
        let reader = try AVAssetReader(asset: asset)
        let output = AVAssetReaderTrackOutput(track: track, outputSettings: [
            AVFormatIDKey: kAudioFormatLinearPCM,
            AVLinearPCMIsFloatKey: true,
            AVLinearPCMBitDepthKey: 32,
            AVLinearPCMIsNonInterleaved: false,
            AVLinearPCMIsBigEndianKey: false
        ])
        output.alwaysCopiesSampleData = false
        guard reader.canAdd(output) else { throw CocoaError(.fileReadUnknown) }
        reader.add(output)
        guard reader.startReading() else { throw reader.error ?? CocoaError(.fileReadUnknown) }
        return try await withTaskCancellationHandler {
            var result = [Float](repeating: 0, count: peakCount)
            while let sample = output.copyNextSampleBuffer() {
                try Task.checkCancellation()
                guard let format = CMSampleBufferGetFormatDescription(sample),
                      let description = CMAudioFormatDescriptionGetStreamBasicDescription(format)?.pointee,
                      let block = CMSampleBufferGetDataBuffer(sample), description.mSampleRate > 0 else { continue }
                let channelCount = Int(description.mChannelsPerFrame)
                let byteCount = CMBlockBufferGetDataLength(block)
                let frameCount = CMSampleBufferGetNumSamples(sample)
                guard channelCount > 0, byteCount >= frameCount * channelCount * MemoryLayout<Float>.size else { throw CocoaError(.fileReadCorruptFile) }
                var samples = [Float](repeating: 0, count: byteCount / MemoryLayout<Float>.size)
                let status = samples.withUnsafeMutableBytes { CMBlockBufferCopyDataBytes(block, atOffset: 0, dataLength: byteCount, destination: $0.baseAddress!) }
                guard status == kCMBlockBufferNoErr else { throw CocoaError(.fileReadCorruptFile) }
                let start = CMSampleBufferGetPresentationTimeStamp(sample).seconds
                guard start.isFinite else { continue }
                for frame in 0..<frameCount {
                    let position = (start + Double(frame) / description.mSampleRate) / duration
                    let bucket = max(0, min(peakCount - 1, Int(position * Double(peakCount))))
                    for channel in 0..<channelCount {
                        let value = abs(samples[frame * channelCount + channel])
                        if value.isFinite { result[bucket] = max(result[bucket], value) }
                    }
                }
            }
            try Task.checkCancellation()
            if reader.status == .failed { throw reader.error ?? CocoaError(.fileReadUnknown) }
            let maximum = result.max() ?? 0
            return maximum > 0 ? result.map { pow($0 / maximum, 0.7) } : result
        } onCancel: { reader.cancelReading() }
    }
}
