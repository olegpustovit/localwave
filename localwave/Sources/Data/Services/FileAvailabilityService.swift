import Foundation
import LocalWaveDomain
import LocalWaveCore

public enum FileAvailabilityService {
    public static func requireLocalForNetworkSimulation(_ url: URL, preferences: UserDefaults = LocalWavePreferences.current) throws {
        guard preferences.bool(forKey: "lwSimulateNoNetwork") else { return }
        guard try inspect(url) == .onDevice else { throw URLError(.notConnectedToInternet) }
    }

    /// Resource metadata only: this never opens an audio stream or asks a provider to download it.
    public static func inspect(_ url: URL, knownLocalCopy: Bool = false) throws -> FileAvailability {
        let values = try url.resourceValues(forKeys: [.isUbiquitousItemKey, .ubiquitousItemDownloadingStatusKey, .volumeIsLocalKey, .volumeIsRemovableKey, .volumeIsInternalKey])
        if values.isUbiquitousItem == true {
            switch values.ubiquitousItemDownloadingStatus {
            case .notDownloaded: return .inCloud
            case .downloaded, .current: return .onDevice
            default: return .unknown
            }
        }
        guard FileManager.default.isReadableFile(atPath: url.path) else { return .unknown }
        if knownLocalCopy { return .onDevice }
        let file = url.resolvingSymlinksInPath().standardizedFileURL
        guard !isProviderLocation(file), values.volumeIsLocal == true else { return .unknown }
        if values.volumeIsRemovable == true || values.volumeIsInternal == false || file.pathComponents.dropFirst().first == "Volumes" { return .onDevice }
        let manager = FileManager.default
        let localRoots = [FileManager.SearchPathDirectory.documentDirectory, .downloadsDirectory, .desktopDirectory, .musicDirectory].flatMap { manager.urls(for: $0, in: .userDomainMask) } + [manager.temporaryDirectory]
        if localRoots.contains(where: { contains(file, under: $0) }) { return .onDevice }
        let components = file.pathComponents
        if let users = components.firstIndex(of: "Users"), components.count > users + 2,
           ["Music", "Documents", "Desktop", "Downloads", "Movies", "Public"].contains(components[users + 2]) { return .onDevice }
        if let applications = components.firstIndex(of: "Application"), components.count > applications + 2,
           UUID(uuidString: components[applications + 1]) != nil, components[applications + 2] == "Documents" { return .onDevice }
        return .unknown
    }

    public static func isProviderLocation(_ url: URL) -> Bool {
        let components = url.resolvingSymlinksInPath().standardizedFileURL.pathComponents
        let storage = Set(["CloudStorage", "Mobile Documents", "CloudDocs", "FileProviderStorage", "File Provider Storage", "FileProvider"])
        for index in components.indices.dropLast() where components[index] == "Library" {
            if storage.contains(components[index + 1]) { return true }
        }
        return (components.contains("AppGroup") || components.contains("PluginKitPlugin")) &&
            (components.contains("FileProviderStorage") || components.contains("File Provider Storage"))
    }

    private static func contains(_ file: URL, under root: URL) -> Bool {
        let rootPath = root.resolvingSymlinksInPath().standardizedFileURL.path
        return file.path == rootPath || file.path.hasPrefix(rootPath.hasSuffix("/") ? rootPath : rootPath + "/")
    }
}
