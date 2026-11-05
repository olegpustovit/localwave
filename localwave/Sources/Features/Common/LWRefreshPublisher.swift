import Combine
import Foundation

enum LWRefresh {
    /// Song or library changes, coalesced. Kept out of view bodies so they stay cheap to type-check.
    static var libraryChanges: AnyPublisher<Void, Never> {
        NotificationCenter.default.publisher(for: Notification.Name("SongListRefresh"))
            .merge(with: NotificationCenter.default.publisher(for: Notification.Name("LibraryRefresh")))
            .debounce(for: .milliseconds(150), scheduler: RunLoop.main)
            .map { _ in () }
            .eraseToAnyPublisher()
    }
}
