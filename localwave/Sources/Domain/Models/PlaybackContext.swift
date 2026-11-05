import Foundation

public enum PlaybackContext: Codable, Hashable, Sendable {
    case library
    case album(name: String, artist: String)
    case artist(String)
    case playlist(id: Int64, name: String)
    case folder(sourceID: Int64, pathID: Int64?, name: String)
    case automatic(String)
    case selection(String)

    public var title: String {
        switch self {
        case .library: return "Library"
        case .album(let name, _), .artist(let name), .playlist(_, let name),
             .folder(_, _, let name), .automatic(let name), .selection(let name): return name
        }
    }
}
