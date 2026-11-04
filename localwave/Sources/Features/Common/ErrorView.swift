import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  ErrorView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct ErrorView: View {
    let error: String

    var body: some View {
        VStack {
            Text("Initialization Error")
                .font(.title)
            Text(error)
                .foregroundColor(.red)
                .padding()
        }
    }
}
