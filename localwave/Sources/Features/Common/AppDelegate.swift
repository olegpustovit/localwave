import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  AppDelegate.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

class AppDelegate: NSObject, UIApplicationDelegate {
    let logger = Logger(subsystem: subsystem, category: "AppDelegate")
    func application(
        _: UIApplication,
        didFinishLaunchingWithOptions _: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        do {
            try AVAudioSession.sharedInstance().setCategory(.playback)
            try AVAudioSession.sharedInstance().setActive(true)
        } catch {
            logger.error("Audio session setup error: \(error)")
        }
        return true
    }
}
