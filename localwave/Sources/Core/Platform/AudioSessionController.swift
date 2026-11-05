import Foundation
#if canImport(UIKit) && canImport(AVFoundation)
import AVFoundation
#endif

public actor AudioSessionController {
    public enum Event: Sendable, Equatable {
        case interruptionBegan
        case interruptionEnded(shouldResume: Bool)
        case oldOutputUnavailable
    }

    public static let shared = AudioSessionController()
    private let configureSession: @Sendable () throws -> Void
    private let activateSession: @Sendable () throws -> Void
    private let observesSystemEvents: Bool
    private var configured = false
    private var active = false
    private var observers: [NSObjectProtocol] = []
    private var listeners: [UUID: AsyncStream<Event>.Continuation] = [:]

    private init() {
        configureSession = {
            #if canImport(UIKit) && canImport(AVFoundation)
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
            #endif
        }
        activateSession = {
            #if canImport(UIKit) && canImport(AVFoundation)
            try AVAudioSession.sharedInstance().setActive(true)
            #endif
        }
        observesSystemEvents = true
    }

    init(configure: @escaping @Sendable () throws -> Void, activate: @escaping @Sendable () throws -> Void) {
        configureSession = configure
        activateSession = activate
        observesSystemEvents = false
    }

    // Actor isolation keeps blocking system calls serialized and off the UI executor.
    public func activate() throws {
        try Task.checkCancellation()
        installObservers()
        if !configured { try configureSession(); configured = true }
        if !active { try activateSession(); active = true }
        try Task.checkCancellation()
    }

    public func events() -> AsyncStream<Event> {
        installObservers()
        let id = UUID()
        return AsyncStream(bufferingPolicy: .bufferingNewest(8)) { continuation in
            listeners[id] = continuation
            continuation.onTermination = { [weak self] _ in
                Task { await self?.removeListener(id) }
            }
        }
    }

    func receive(_ event: Event) {
        if event == .interruptionBegan { active = false }
        for continuation in listeners.values { continuation.yield(event) }
    }

    private func removeListener(_ id: UUID) { listeners.removeValue(forKey: id) }

    private func installObservers() {
        guard observesSystemEvents, observers.isEmpty else { return }
        #if canImport(UIKit) && canImport(AVFoundation)
        let center = NotificationCenter.default
        observers.append(center.addObserver(forName: AVAudioSession.interruptionNotification, object: nil, queue: nil) { [weak self] notification in
            guard let raw = notification.userInfo?[AVAudioSessionInterruptionTypeKey] as? UInt,
                  let kind = AVAudioSession.InterruptionType(rawValue: raw) else { return }
            let event: Event
            if kind == .began { event = .interruptionBegan }
            else {
                let options = (notification.userInfo?[AVAudioSessionInterruptionOptionKey] as? UInt) ?? 0
                event = .interruptionEnded(shouldResume: AVAudioSession.InterruptionOptions(rawValue: options).contains(.shouldResume))
            }
            Task { await self?.receive(event) }
        })
        observers.append(center.addObserver(forName: AVAudioSession.routeChangeNotification, object: nil, queue: nil) { [weak self] notification in
            guard let raw = notification.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt,
                  AVAudioSession.RouteChangeReason(rawValue: raw) == .oldDeviceUnavailable else { return }
            Task { await self?.receive(.oldOutputUnavailable) }
        })
        #endif
    }

    deinit { for observer in observers { NotificationCenter.default.removeObserver(observer) } }
}
