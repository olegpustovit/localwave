//
//  SelectableSongRow.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct SelectableSongRow: View {
    let song: Song
    let isSelected: Bool
    let onToggle: () -> Void

    @State var isFavorite: Bool = false
    
    var body: some View {
        HStack {
            
            Image(systemName: "star.fill")
                .foregroundColor(isFavorite ? .yellow : .white)
            
            VStack(alignment: .leading) {
                Text(song.title)
                    .font(.headline)
                Text(song.artist)
                    .font(.subheadline)
                    .foregroundColor(.secondary)
            }
            Spacer()
            Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                .foregroundColor(isSelected ? .blue : .gray)
        }
        .contentShape(Rectangle())
        .onTapGesture {
            onToggle()
        }
        .padding(.vertical, 4)
    }
}
