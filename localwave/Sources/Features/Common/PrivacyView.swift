#if os(iOS)
import SwiftUI

struct PrivacyView: View {
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                Text("LocalWave").font(.title.bold())
                Text("A music player for your files.")
                Text("Your library stays in the app’s storage")
                    .font(.headline)
                Text("LocalWave stores song metadata, artwork, playlists, Favorites, playback settings, folder bookmarks, and cached audio locally. It does not send your music or listening history to the developer and includes no advertising or analytics service.")
                Text("Files and iCloud Drive")
                    .font(.headline)
                Text("Files you select from iCloud Drive or another provider may need downloading through that provider. If iCloud is available, a hash of the system’s identity token associates sources with the current account locally. LocalWave does not receive your Apple Account password.")
                Text("Removing your data")
                    .font(.headline)
                Text("Removing a library entry removes its database record; cached audio or artwork may remain. Deleting the app removes its local storage, subject to your backup settings. Your original files in Files and iCloud Drive stay where they are.")
                Link("Full privacy policy", destination: URL(string: "https://github.com/olegpustovit/localwave/blob/dev/docs/privacy.md")!)
                Link("Support", destination: URL(string: "https://github.com/olegpustovit/localwave/issues")!)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding()
        }
        .navigationTitle("About & Privacy")
    }
}

#endif
