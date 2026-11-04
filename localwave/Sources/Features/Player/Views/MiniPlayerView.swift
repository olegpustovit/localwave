import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  MiniPlayerView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct MiniPlayerView: View {
    @EnvironmentObject private var playerVM: PlayerViewModel
    var onTap: () -> Void

    var body: some View {
        MiniPlayerViewInner(
            currentSong: playerVM.currentSong,
            onTap: onTap, playPauseAction: { playerVM.playPause() }, isPlaying: playerVM.isPlaying
        )
    }
}
