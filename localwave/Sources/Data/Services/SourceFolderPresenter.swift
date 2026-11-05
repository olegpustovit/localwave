import Foundation

/// Receives coordinated provider changes without retaining a source-access actor.
public final class SourceFolderPresenter: NSObject, NSFilePresenter, @unchecked Sendable {
    public let presentedItemOperationQueue: OperationQueue
    private let lock = NSLock()
    private var root: URL
    private var active = true
    private var pending: DispatchWorkItem?
    private var movedRoot: URL?
    private var revision = UUID()
    private let delivery = DispatchQueue(label: "localwave.source-presenter")
    private let deliveryKey = DispatchSpecificKey<Bool>()
    private let onChange: @Sendable (URL?) -> Void

    public var presentedItemURL: URL? {
        lock.lock()
        defer { lock.unlock() }
        return root
    }

    public init(url: URL, onChange: @escaping @Sendable (URL?) -> Void) {
        root = url
        self.onChange = onChange
        let operations = OperationQueue()
        operations.name = "localwave.source-presenter.operations"
        operations.maxConcurrentOperationCount = 1
        presentedItemOperationQueue = operations
        super.init()
        delivery.setSpecific(key: deliveryKey, value: true)
        NSFileCoordinator.addFilePresenter(self)
    }

    public func invalidate() {
        lock.lock()
        let wasActive = active
        active = false
        pending?.cancel()
        pending = nil
        movedRoot = nil
        lock.unlock()
        if wasActive {
            NSFileCoordinator.removeFilePresenter(self)
            presentedItemOperationQueue.cancelAllOperations()
        }
        // Finish an in-flight delivery before returning, including invalidation from its callback.
        if DispatchQueue.getSpecific(key: deliveryKey) != true { delivery.sync {} }
    }

    deinit { invalidate() }

    public func presentedItemDidChange() { changed() }
    public func presentedSubitemDidAppear(at url: URL) { changed() }
    public func presentedSubitemDidChange(at url: URL) { changed() }
    public func presentedSubitem(at oldURL: URL, didMoveTo newURL: URL) { changed() }
    public func presentedItemDidMove(to newURL: URL) { changed(movedTo: newURL) }

    public func accommodatePresentedSubitemDeletion(at url: URL, completionHandler: @escaping (Error?) -> Void) {
        changed()
        completionHandler(nil)
    }

    private func changed(movedTo newURL: URL? = nil) {
        lock.lock()
        guard active else { lock.unlock(); return }
        if let newURL { root = newURL; movedRoot = newURL }
        pending?.cancel()
        revision = UUID()
        let event = revision
        let work = DispatchWorkItem { [weak self] in self?.deliver(event) }
        pending = work
        delivery.asyncAfter(deadline: .now() + .milliseconds(500), execute: work)
        lock.unlock()
    }

    private func deliver(_ event: UUID) {
        lock.lock()
        guard active && revision == event else { lock.unlock(); return }
        let moved = movedRoot
        movedRoot = nil
        pending = nil
        lock.unlock()
        onChange(moved)
    }
}
