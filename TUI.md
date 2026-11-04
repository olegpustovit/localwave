# LocalWave TUI Implementation Plan

## Master Checklist

### Phase 1: Foundation & Setup ✅
- [x] 1.1 Create TUI target and basic structure
- [x] 1.2 Add SwiftTUI dependency and shared code infrastructure
- [x] 1.3 Create TUI-specific dependency injection
- [x] 1.4 Build platform abstraction layer

### Phase 2: Core TUI Components ✅
- [x] 2.1 Implement base TUI navigation system
- [x] 2.2 Implement keyboard input with onKeyPress (vim-style)
- [x] 2.3 Create reusable TUI component library
- [x] 2.4 Build TUI theme and styling system

### Phase 3: Library Features - Artists/Albums/Songs
- [x] 3.1 Build artist list TUI view
- [x] 3.2 Build artist detail view (songs by artist)
- [x] 3.3 Build album grid TUI view
- [x] 3.4 Build album detail view (album songs)
- [x] 3.5 Build all songs list TUI view
- [x] 3.6 Implement search functionality
- [x] 3.7 Implement song metadata editor

### Phase 4: Player Features
- [x] 4.1 Create mini player status bar
- [x] 4.2 Create full player view (Now Playing)
- [x] 4.3 Implement playback controls (seek, shuffle, repeat)
- [x] 4.4 Build queue management (add/remove/reorder/clear)
- [x] 4.5 Add progress bar and time display
- [x] 4.6 Implement shuffle/repeat modes

### Phase 5: Playlist Management
- [ ] 5.1 Build playlist list view
- [ ] 5.2 Implement playlist detail view
- [ ] 5.3 Add playlist creation/editing/deletion
- [ ] 5.4 Build song selection for playlists
- [ ] 5.5 Implement add to playlist flow

### Phase 6: Sync & Source Management
- [x] 6.1 Build sync view (source browser)
- [x] 6.2 Implement source selection and scanning
- [x] 6.3 Add sync progress display
- [x] 6.4 Build source management (add/remove sources)

### Phase 7: Integration & Polish
- [ ] 7.1 Connect all TUI views with navigation
- [ ] 7.2 Add help system and keyboard shortcuts overlay
- [ ] 7.3 Implement status bar and notifications
- [ ] 7.4 Add error handling and recovery
- [ ] 7.5 Performance optimization and polish

---

## iOS Feature Parity Mapping

Based on iOS app structure, TUI must implement:

### Library Features
- **Artists Tab**: List all artists → Artist detail (songs by artist)
- **Albums Tab**: Grid of albums → Album detail (album songs)
- **Songs Tab**: All songs list with search and filters
- **Song Actions**: Play, add to queue, add to playlist, edit metadata

### Player Features
- **Mini Player**: Always visible at bottom, shows current song
- **Full Player**: Dedicated view with artwork, controls, queue
- **Playback Controls**: Play/pause, next, previous, seek, volume
- **Queue Management**: View queue, reorder, clear
- **Shuffle/Repeat**: Toggle shuffle and repeat modes (off/all/one)

### Playlist Features
- **Playlist List**: View all playlists
- **Playlist Detail**: View songs in playlist, play, edit
- **Create Playlist**: New playlist with name
- **Add to Playlist**: Select songs to add to playlist
- **Manage Playlists**: Rename, delete, reorder songs

### Sync Features
- **Source Browser**: Browse iCloud/local folders
- **Source Selection**: Select folder to scan
- **Sync Progress**: Show scanning progress
- **Source Management**: Add/remove music sources

### Common Features
- **Search**: Full-text search across songs (FTS5)
- **Metadata Editing**: Edit song title, artist, album
- **Error Handling**: User-friendly error messages
- **Theme**: Consistent color scheme and styling

---

## Phase 1: Foundation & Setup ✅

### Task 1.1: Create TUI Target and Basic Structure ✅
**Status**: Complete
**Files created:**
- `localwave-tui/Sources/main.swift`
- `localwave-tui/Sources/App/TUIApp.swift`

### Task 1.2: Add SwiftTUI Dependency ✅
**Status**: Complete
**Files modified:**
- `Package.swift` - Added SwiftTUI dependency and localwave-tui target

### Task 1.3: Create TUI-Specific Dependency Injection ✅
**Status**: Complete
**Files created:**
- `localwave-tui/Sources/App/TUIDependencyContainer.swift`

### Task 1.4: Build Platform Abstraction Layer ✅
**Status**: Complete
**Files created:**
- `localwave/Sources/Core/Platform/PlatformTypes.swift`
- `localwave/Sources/Core/Platform/AudioPlayerProtocol.swift`
- iOS/TUI-specific implementations

---

## Phase 2: Core TUI Components

### Task 2.1: Implement Base TUI Navigation System ✅
**Status**: Complete
**Files created:**
- `localwave-tui/Sources/TUIComponents/Navigation/NavigationStack.swift` (130 lines)
- `localwave-tui/Sources/TUIComponents/Navigation/TabBar.swift` (150 lines)
- `localwave-tui/Sources/App/TUIMainView.swift` (90 lines)

**Implementation:**
- NavigationState struct with stack-based routing
- TabBarState with 5 tabs and keyboard shortcuts
- NavigationContainer and TUITabView integration
- Breadcrumb display
- Uses SwiftTUI @State/@Binding (no Combine support)

---

### Task 2.2: Implement Keyboard Input with onKeyPress ✅
**Status**: Complete - Using DandyLyons/SwiftTUI fork

**Updated dependency:**
- Switched to `https://github.com/DandyLyons/SwiftTUI.git` branch `onKeyPress`
- Provides `.onKeyPress(char:action:)` modifier for key handling

**Implemented keybindings:**

**Tab Navigation:**
- `1-5`: Direct tab switching (Artists, Albums, Songs, Playlists, Player)
- `H`: Previous tab (vim-style)
- `L`: Next tab (vim-style)

**Global shortcuts (placeholders for Phase 7):**
- `q`: Quit application
- `?`: Show help
- `/`: Search

**Vim-style philosophy:**
All navigation follows vim conventions:
- `hjkl`: Movement (h=left, j=down, k=up, l=right)
- `H/L`: Tab navigation (horizontal movement)
- Capital letters for "bigger" movements
- Lowercase for granular control

**API usage:**
```swift
.onKeyPress("1") { tabState.selectTab(.artists) }
.onKeyPress("H") { selectPreviousTab() }
```

---

### Task 2.3: Create Reusable TUI Component Library
**Files to create:**
- `localwave-tui/Sources/TUIComponents/UI/TUIList.swift` (200 lines)
- `localwave-tui/Sources/TUIComponents/UI/TUITable.swift` (180 lines)
- `localwave-tui/Sources/TUIComponents/UI/TUIProgressBar.swift` (100 lines)
- `localwave-tui/Sources/TUIComponents/UI/TUITextField.swift` (120 lines)

**Description:**
- Build scrollable list component with arrow key navigation
- Create table view for multi-column data display
- Implement progress bar using Unicode blocks
- Create text input field with cursor
- All components support keyboard-only interaction

**Key patterns:**
```swift
struct TUIList<Item: Identifiable, Content: View>: View {
    let items: [Item]
    @Binding var selectedItem: Item?
    @ViewBuilder let content: (Item, Bool) -> Content
    @State private var focusedIndex: Int = 0

    var body: some View {
        VStack {
            ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                content(item, index == focusedIndex)
            }
        }
        .onKey(.arrowDown) { focusedIndex = min(focusedIndex + 1, items.count - 1) }
        .onKey(.arrowUp) { focusedIndex = max(focusedIndex - 1, 0) }
        .onKey(.enter) { selectedItem = items[focusedIndex] }
    }
}
```

---

### Task 2.4: Build TUI Theme and Styling System
**Files to create:**
- `localwave-tui/Sources/TUIComponents/Theme/TUITheme.swift` (120 lines)
- `localwave-tui/Sources/TUIComponents/Theme/TUIColors.swift` (80 lines)

**Description:**
- Define standard color palette (no color modifiers in SwiftTUI)
- Create text formatting helpers (icons, spacing)
- Define layout constants (widths, heights)
- Build box-drawing utilities for borders

**Theme elements:**
```swift
struct TUITheme {
    // Icons (use Unicode)
    static let playIcon = "▶"
    static let pauseIcon = "⏸"
    static let musicIcon = "♫"
    static let albumIcon = "◉"
    static let playlistIcon = "☰"

    // Layout
    static let tabBarHeight = 1
    static let miniPlayerHeight = 2
    static let dividerChar = "─"

    // Borders
    static func box(_ content: String) -> String {
        // Box-drawing characters
    }
}
```

---

## Phase 3: Library Features

### Task 3.1: Build Artist List TUI View
**Files to create:**
- `localwave-tui/Sources/Features/Library/TUIArtistListView.swift` (180 lines)

**Description:**
- Display scrollable list of artists using TUIList
- Show song count per artist
- Reuse ArtistListViewModel from main app
- Navigate to artist detail on Enter
- Support search/filter

**UI mockup:**
```
LocalWave > Artists

 > The Beatles                              (142 songs)
   Pink Floyd                                (98 songs)
   Led Zeppelin                              (87 songs)
   David Bowie                               (76 songs)

────────────────────────────────────────────────────────
[↑↓] Navigate  [Enter] Open  [/] Search  [1-5] Tabs
```

**Implementation:**
```swift
struct TUIArtistListView: View {
    @StateObject var viewModel: ArtistListViewModel
    @Binding var navigationState: NavigationState
    @State private var selectedArtist: String?

    var body: some View {
        VStack {
            TUIList(items: viewModel.artists, selectedItem: $selectedArtist) { artist, focused in
                HStack {
                    Text(focused ? "> " : "  ")
                    Text(artist)
                    Spacer()
                    Text("(\(viewModel.songCount(for: artist)) songs)")
                }
            }
            .onSelection { artist in
                navigationState.push(.artistDetail(artist: artist))
            }
        }
    }
}
```

---

### Task 3.2: Build Artist Detail View
**Files to create:**
- `localwave-tui/Sources/Features/Library/TUIArtistDetailView.swift` (200 lines)

**Description:**
- Show songs by selected artist
- Display artist name as header
- Reuse SongListViewModel with artist filter
- Support playing songs, adding to queue/playlist
- Show song count and total duration

**UI mockup:**
```
LocalWave > Artists > The Beatles

 The Beatles - 142 songs - 8h 23m

 ♫ Come Together                Abbey Road           4:20
   Something                    Abbey Road           3:03
   Here Comes The Sun           Abbey Road           3:06
   ...

────────────────────────────────────────────────────────
[↑↓] Select  [Space] Play  [Q] Queue  [P] Playlist  [Esc] Back
```

---

### Task 3.3: Build Album Grid TUI View
**Files to create:**
- `localwave-tui/Sources/Features/Library/TUIAlbumGridView.swift` (220 lines)

**Description:**
- Display albums in grid/list format
- Show album name, artist, year, song count
- Reuse AlbumListViewModel
- Support sorting by name/artist/year
- Navigate to album detail

**UI mockup:**
```
LocalWave > Albums

 Album                Artist              Year    Songs
────────────────────────────────────────────────────────
 > Abbey Road         The Beatles         1969    17
   Dark Side...       Pink Floyd          1973    10
   Led Zeppelin IV    Led Zeppelin        1971    8
   Ziggy Stardust     David Bowie         1972    11

────────────────────────────────────────────────────────
[↑↓] Navigate  [Enter] Open  [S] Sort  [/] Search
```

---

### Task 3.4: Build Album Detail View
**Files to create:**
- `localwave-tui/Sources/Features/Library/TUIAlbumDetailView.swift` (200 lines)

**Description:**
- Show album info (name, artist, year)
- List all songs in album with track numbers
- Reuse SongListViewModel with album filter
- Support playing album, adding to queue
- Show total duration

**UI mockup:**
```
LocalWave > Albums > Abbey Road

 Abbey Road - The Beatles (1969) - 17 songs - 47:23

  1. Come Together                               4:20
  2. Something                                   3:03
  3. Maxwell's Silver Hammer                     3:27
  ...

────────────────────────────────────────────────────────
[Space] Play Album  [Q] Queue All  [Esc] Back
```

---

### Task 3.5: Build All Songs List TUI View
**Files to create:**
- `localwave-tui/Sources/Features/Library/TUISongListView.swift` (250 lines)

**Description:**
- Display all songs in table format
- Columns: Title, Artist, Album, Duration
- Reuse SongListViewModel with .all filter
- Show currently playing indicator (♫)
- Support multi-select for batch operations

**UI mockup:**
```
LocalWave > Songs

 Title                Artist           Album            Time
────────────────────────────────────────────────────────────
 ♫ Come Together      The Beatles      Abbey Road       4:20
   Something          The Beatles      Abbey Road       3:03
   Comfortably Numb   Pink Floyd       The Wall         6:23
   ...

────────────────────────────────────────────────────────────
[↑↓] Select  [Space] Play  [/] Search  [E] Edit
```

---

### Task 3.6: Implement Search Functionality
**Files to create:**
- `localwave-tui/Sources/Features/Library/TUISearchView.swift` (200 lines)

**Description:**
- Overlay search activated by `/` key
- Use FTS5 search via SongRepository
- Display results in real-time as user types
- Support navigation to results
- Show match count

**UI mockup:**
```
LocalWave > Songs

┌─ Search ───────────────────────────┐
│ Query: [come_____________________] │
│                                    │
│ 23 results found:                  │
│  > Come Together - The Beatles     │
│    Come As You Are - Nirvana       │
│    Here Comes The Sun - Beatles    │
│    ...                             │
└────────────────────────────────────┘

[Type] Search  [↑↓] Select  [Enter] Open  [Esc] Close
```

---

### Task 3.7: Implement Song Metadata Editor
**Files to create:**
- `localwave-tui/Sources/Features/Library/TUISongEditorView.swift` (180 lines)

**Description:**
- Modal form to edit song metadata
- Fields: Title, Artist, Album, Year
- Validation and save/cancel
- Update database and refresh list

**UI mockup:**
```
┌─ Edit Song ─────────────────────────────┐
│                                          │
│  Title:  [Come Together_____________]   │
│  Artist: [The Beatles_______________]   │
│  Album:  [Abbey Road________________]   │
│  Year:   [1969______]                   │
│                                          │
│       [Enter] Save    [Esc] Cancel      │
└──────────────────────────────────────────┘
```

---

## Phase 4: Player Features

### Task 4.1: Create Mini Player Status Bar ✅
**Status**: Complete

**Files created:**
- `localwave-tui/Sources/Features/Player/TUIMiniPlayerView.swift` (70 lines)
- `localwave-tui/Sources/Core/TUIMockAudioPlayer.swift` (43 lines)

**Implementation:**
- Always visible 2-line status bar at bottom of screen
- Line 1: Play/pause icon, artist - title, shuffle icon, repeat symbol
- Line 2: Progress bar with current time / duration
- Uses @ObservedObject BasePlayerViewModel for real-time updates
- @MainActor isolated for safe access to player state
- TUIMockAudioPlayer implements AudioPlayerProtocol for testing

**Integration:**
- Added to TUIMainView bottom via VStack wrapper
- Visible across all tabs and navigation states
- BasePlayerViewModel initialized in TUIDependencyContainer

**UI implemented:**
```
────────────────────────────────────────────────────────
▶ The Beatles - Come Together                    🔀 ↻
[████████──────────────] 2:30 / 4:20
```

---

### Task 4.2: Create Full Player View ✅
**Status**: Complete

**Files created:**
- `localwave-tui/Sources/Features/Player/TUIPlayerView.swift` (282 lines)

**Implementation:**
- Dedicated player view accessible via Tab 6
- Large song info display (title, artist, album, year)
- Playback controls: ◀◀ ⏸/▶ ▶▶
- Volume control with visual progress bar (0-100%)
- Shuffle toggle and Repeat mode display
- Queue preview showing next 3 songs
- Keyboard controls: Space (play/pause), P/N (prev/next), [/] (seek), +/- (volume), S (shuffle), R (repeat)
- All action methods @MainActor for safe player interaction
- View components extract @Published properties to avoid actor isolation errors

**Integration:**
- Replaced PlayerPlaceholder in TUIMainView Tab 6
- Shares BasePlayerViewModel with mini player for consistent state

**Modified shared code:**
- BasePlayerViewModel: Made isShuffleEnabled, repeatMode, play(), pause() public for TUI access
- Queue preview (next 3 songs)

**UI mockup:**
```
LocalWave > Player

┌─ Now Playing ─────────────────────────────────┐
│                                                │
│           🎵  Come Together                   │
│               The Beatles                      │
│               Abbey Road (1969)                │
│                                                │
│   [████████████████──────────────]            │
│          2:30 / 4:20                          │
│                                                │
│      [◀◀]     [⏸]     [▶▶]                   │
│                                                │
│   Shuffle: 🔀   Repeat: 🔁                    │
│                                                │
└────────────────────────────────────────────────┘

Up Next:
 1. Something - The Beatles
 2. Here Comes The Sun - The Beatles
 3. Maxwell's Silver Hammer - The Beatles

────────────────────────────────────────────────────
[Space] Play/Pause  [←→] Prev/Next  [Q] Queue
```

---

### Task 4.3: Implement Playback Controls ✅
**Status**: Complete

**Files modified:**
- `localwave-tui/Sources/Features/Player/TUIPlayerView.swift`
- `localwave/Sources/Features/Player/ViewModels/BasePlayerViewModel.swift`

**Implementation:**
- Added seekBySeconds(_:) to BasePlayerViewModel for -10s/+10s seeking
- Added toggleShuffle() and toggleRepeat() convenience methods
- Wired up all keyboard shortcuts to player controls
- All action methods properly @MainActor isolated

**Keyboard shortcuts implemented:**
- `Space`: Play/Pause (togglePlayPause)
- `P` / `N`: Previous/Next song
- `[` / `]`: Seek -10s / +10s
- `+` / `-`: Volume up/down (10% increments)
- `S`: Toggle shuffle on/off
- `R`: Cycle repeat mode (none → all → one)
- `Q`: Open queue management view

---

### Task 4.4: Build Queue Management ✅
**Status**: Complete

**Files created:**
- `localwave-tui/Sources/Features/Player/TUIQueueView.swift` (328 lines)

**Files modified:**
- `localwave/Sources/Features/Player/ViewModels/BasePlayerViewModel.swift` (added queue methods)
- `localwave-tui/Sources/TUIComponents/Navigation/NavigationStack.swift` (added .queue route)
- `localwave-tui/Sources/App/TUIMainView.swift` (route handler)

**Implementation:**
- Display current playback queue with song index, artist, title
- Highlight currently playing song with ♫ indicator
- Queue stats: song count and estimated total duration
- Vim-style navigation: j/k (up/down), g/G (first/last), u/d (page up/down)
- Pagination: 10 items per page with auto-scroll
- Remove songs: x key or Backspace (protects currently playing song)
- Clear queue: c key (stops playback, clears all)
- Reorder songs: K/J (Shift+K/J) to move up/down
- Auto-centers on currently playing song when opened
- Safe operations: prevents moving/removing currently playing song

**BasePlayerViewModel queue methods:**
- `removeFromQueue(at:)` - removes song, adjusts current index
- `clearQueue()` - stops playback, clears all songs
- `moveSong(from:to:)` - reorders queue using Int indices (TUI-compatible)
- All operations persist via PlayerPersistenceService

**UI implemented:**
```
LocalWave > Queue

 Queue (5 songs - 15:00)

   1. Something - The Beatles
   2. Here Comes The Sun - The Beatles
 ♫ 3. Come Together - The Beatles  ◀ Now Playing
   4. Maxwell's Silver Hammer - The Beatles
   5. Oh! Darling - The Beatles

────────────────────────────────────────────────────────
[j/k] Navigate  [K/J] Move Up/Down  [x] Remove  [c] Clear
[Esc] Back
```

---

### Task 4.5: Add Progress Bar and Time Display
**Files to create:**
- `localwave-tui/Sources/TUIComponents/UI/TUIProgressBar.swift` (120 lines)

**Description:**
- Smooth progress bar using Unicode blocks: `▏▎▍▌▋▊▉█`
- Display elapsed / total time
- Support keyboard seeking
- Update every second
- Visual feedback during seeking

**Progress bar states:**
```
Playing:  [████████──────────] 2:30 / 4:20
Seeking:  [████████━━━━━━━━━━] 2:45 / 4:20  (seeking...)
Buffering: [████████▒▒▒▒▒▒▒▒▒▒] 2:30 / 4:20  (loading...)
```

---

### Task 4.6: Implement Shuffle/Repeat Modes
**Files to modify:**
- `localwave-tui/Sources/Features/Player/TUIPlayerView.swift`
- Add toggle controls for shuffle and repeat

**Features:**
- Shuffle toggle: Off / On (🔀)
- Repeat toggle: Off / All (🔁) / One (🔂)
- Keyboard shortcuts: `S` shuffle, `R` cycle repeat
- Visual indicators in player and mini player
- Persist state in PlayerViewModel

**UI indicators:**
```
Shuffle OFF, Repeat OFF:  [    ]
Shuffle ON,  Repeat OFF:  [ 🔀 ]
Shuffle OFF, Repeat ALL:  [ 🔁 ]
Shuffle OFF, Repeat ONE:  [ 🔂 ]
```

---

## Phase 5: Playlist Management

### Task 5.1: Build Playlist List View
**Files to create:**
- `localwave-tui/Sources/Features/Playlists/TUIPlaylistListView.swift` (180 lines)

**Description:**
- Display all playlists with song counts
- Reuse PlaylistListViewModel
- Support create, delete, rename
- Navigate to playlist detail
- Show last modified date

**UI mockup:**
```
LocalWave > Playlists

 > Favorites                        45 songs - Modified 2 days ago
   Workout Mix                      23 songs - Modified 1 week ago
   Chill Vibes                      67 songs - Modified 3 weeks ago
   Road Trip                        89 songs - Modified 1 month ago

────────────────────────────────────────────────────────────────
[↑↓] Select  [Enter] Open  [N] New  [R] Rename  [Del] Delete
```

---

### Task 5.2: Implement Playlist Detail View
**Files to create:**
- `localwave-tui/Sources/Features/Playlists/TUIPlaylistDetailView.swift` (220 lines)

**Description:**
- Show playlist name and metadata
- List songs in playlist with positions
- Reuse PlaylistDetailViewModel
- Support playing playlist, shuffling
- Remove/reorder songs
- Add more songs

**UI mockup:**
```
LocalWave > Playlists > Favorites

 Favorites - 45 songs - 3h 12m

  1. Come Together - The Beatles                 4:20
  2. Wish You Were Here - Pink Floyd             5:34
  3. Stairway to Heaven - Led Zeppelin           8:02
  ...

──────────────────────────────────────────────────────
[Space] Play All  [S] Shuffle  [A] Add Songs  [Del] Remove
```

---

### Task 5.3: Add Playlist Creation/Editing
**Files to create:**
- `localwave-tui/Sources/Features/Playlists/TUIPlaylistEditorView.swift` (150 lines)

**Description:**
- Modal form for playlist name
- Create new or rename existing
- Validation (non-empty, unique name)
- Save/cancel actions
- Focus on text field

**UI mockups:**
```
┌─ Create Playlist ──────────────────┐
│                                     │
│  Name: [My Playlist____________]   │
│                                     │
│    [Enter] Create  [Esc] Cancel    │
└─────────────────────────────────────┘

┌─ Rename Playlist ──────────────────┐
│                                     │
│  New name: [Favorites__________]   │
│                                     │
│    [Enter] Save  [Esc] Cancel      │
└─────────────────────────────────────┘
```

---

### Task 5.4: Build Song Selection for Playlists
**Files to create:**
- `localwave-tui/Sources/Features/Playlists/TUISongSelectionView.swift` (240 lines)

**Description:**
- Multi-select song list
- Search/filter songs
- Visual selection indicators [✓] / [ ]
- Add selected songs to playlist
- Select all/none shortcuts

**UI mockup:**
```
LocalWave > Playlists > Favorites > Add Songs

 Add Songs to Playlist (3 selected)

 [✓] Come Together - The Beatles
 [ ] Something - The Beatles
 [✓] Comfortably Numb - Pink Floyd
 [✓] Stairway to Heaven - Led Zeppelin
 [ ] Heroes - David Bowie

──────────────────────────────────────────────────────
[Space] Toggle  [A] Select All  [N] Select None  [Enter] Add
```

---

### Task 5.5: Implement Add to Playlist Flow
**Files to create:**
- `localwave-tui/Sources/Features/Playlists/TUIAddToPlaylistView.swift` (180 lines)

**Description:**
- Quick playlist selector for adding songs
- Show all playlists
- Support creating new playlist inline
- Add song(s) to selected playlist
- Confirmation message

**UI mockup:**
```
┌─ Add to Playlist ──────────────────┐
│                                     │
│  Select playlist:                  │
│   > Favorites                       │
│     Workout Mix                     │
│     Chill Vibes                     │
│     [+ Create New Playlist]         │
│                                     │
│  [Enter] Add  [Esc] Cancel         │
└─────────────────────────────────────┘

✓ Added "Come Together" to "Favorites"
```

---

## Phase 6: Sync & Source Management

### Task 6.1: Build Sync View (Source Browser)
**Files to create:**
- `localwave-tui/Sources/Features/Sync/TUISyncView.swift` (200 lines)

**Description:**
- Browse iCloud/local folders
- Show folder tree structure
- Reuse SourceBrowseViewModel
- Display folder stats (# of MP3s)
- Select folder to add as source

**UI mockup:**
```
LocalWave > Sync

 Browse for Music Folders

 📁 iCloud Drive
   > 📁 Music
       📁 The Beatles (142 files)
       📁 Pink Floyd (98 files)
       📁 Led Zeppelin (87 files)
 📁 Local Files
   📁 Downloads

────────────────────────────────────────────────────────
[↑↓] Navigate  [→] Expand  [←] Collapse  [Enter] Select
```

---

### Task 6.2: Implement Source Selection and Scanning
**Files to create:**
- `localwave-tui/Sources/Features/Sync/TUISourceScanView.swift` (180 lines)

**Description:**
- Show selected folder path
- Display scanning progress
- Show files being processed
- Reuse sync service logic
- Cancel option

**UI mockup:**
```
LocalWave > Sync > Scanning

 Scanning: /iCloud Drive/Music/The Beatles

 Progress: [████████████──────] 75% (106/142 files)

 Processing: Abbey Road/03 - Maxwell's Silver Hammer.mp3

 Added: 106 songs
 Updated: 3 songs
 Errors: 0

────────────────────────────────────────────────────────
[Esc] Cancel Scan
```

---

### Task 6.3: Add Sync Progress Display
**Files to create:**
- `localwave-tui/Sources/Features/Sync/TUISyncProgressView.swift` (150 lines)

**Description:**
- Real-time progress updates
- Show current file being processed
- Display stats (added/updated/errors)
- Estimate time remaining
- Completion summary

**Progress states:**
```
Scanning...
[████──────] 40% - 2m remaining
Processing: The Beatles/Abbey Road/01 - Come Together.mp3

Complete!
✓ Added 142 new songs
✓ Updated 5 existing songs
✗ 2 errors (see log)

[Enter] Done
```

---

### Task 6.4: Build Source Management
**Files to create:**
- `localwave-tui/Sources/Features/Sync/TUISourceManagementView.swift` (180 lines)

**Description:**
- List all music sources
- Show source path and song count
- Add new source
- Remove source (with confirmation)
- Rescan source

**UI mockup:**
```
LocalWave > Sync > Sources

 Music Sources (3)

 > 📁 /iCloud Drive/Music/The Beatles    142 songs
   📁 /iCloud Drive/Music/Pink Floyd      98 songs
   📁 /Local/Music                        234 songs

────────────────────────────────────────────────────────
[↑↓] Select  [A] Add  [Del] Remove  [R] Rescan  [Esc] Back
```

---

## Phase 7: Integration & Polish

### Task 7.1: Connect All TUI Views with Navigation
**Files to modify:**
- `localwave-tui/Sources/App/TUIMainView.swift` (expand routing logic)

**Description:**
- Complete NavigationRoute enum with all routes
- Wire all feature views to navigation
- Implement breadcrumb updates
- Support deep navigation (Artists → Artist → Album → Song)
- Handle back navigation (Esc)

**Route structure:**
```swift
enum NavigationRoute {
    case artists
    case artistDetail(artist: String)
    case albums
    case albumDetail(album: String, artist: String?)
    case songs
    case songEditor(songId: Int64)
    case playlists
    case playlistDetail(playlistId: Int64)
    case playlistAddSongs(playlistId: Int64)
    case player
    case queue
    case sync
    case syncScan(source: String)
    case search(query: String)
}
```

---

### Task 7.2: Add Help System and Keyboard Shortcuts
**Files to create:**
- `localwave-tui/Sources/Features/Help/TUIHelpView.swift` (200 lines)
- `localwave-tui/Sources/Features/Help/TUIShortcutRegistry.swift` (150 lines)

**Description:**
- Help overlay activated with `?`
- Document all keyboard shortcuts
- Context-aware help (show relevant shortcuts)
- Searchable command palette
- Organized by category

**UI mockup:**
```
┌─ Help ───────────────────────────────────────────┐
│                                                   │
│  Global Shortcuts:                               │
│    ?        Show this help                       │
│    Q        Quit application                     │
│    /        Search                               │
│    1-5      Switch tabs                          │
│                                                   │
│  Navigation:                                     │
│    ↑↓       Select item                          │
│    Enter    Open/Confirm                         │
│    Esc      Back/Cancel                          │
│                                                   │
│  Player:                                         │
│    Space    Play/Pause                           │
│    →/←      Next/Previous                        │
│    S        Toggle Shuffle                       │
│    R        Cycle Repeat Mode                    │
│                                                   │
│  [Esc] Close Help                                │
└───────────────────────────────────────────────────┘
```

---

### Task 7.3: Implement Status Bar and Notifications
**Files to create:**
- `localwave-tui/Sources/TUIComponents/UI/TUIStatusBar.swift` (120 lines)
- `localwave-tui/Sources/TUIComponents/UI/TUIToast.swift` (100 lines)

**Description:**
- Top status bar with breadcrumb
- Toast notifications for actions
- Auto-dismiss after 3 seconds
- Queue multiple toasts

**Status bar layout:**
```
LocalWave > Artists > The Beatles       142 songs · Queue: 5
```

**Toast examples:**
```
✓ Added to queue: Come Together - The Beatles
✓ Playlist "Favorites" created
✗ Error: Could not load file
```

---

### Task 7.4: Add Error Handling and Recovery
**Files to create:**
- `localwave-tui/Sources/Core/TUIErrorHandler.swift` (180 lines)
- `localwave-tui/Sources/TUIComponents/UI/TUIErrorView.swift` (120 lines)

**Description:**
- Centralized error handling
- User-friendly error messages
- Retry actions where applicable
- Log errors for debugging
- Graceful degradation

**Error scenarios:**
- Database connection failures → Retry connection
- Audio file not found → Skip and continue
- Invalid input → Show validation message
- Sync errors → Show error count, offer to continue

**UI mockup:**
```
┌─ Error ────────────────────────────────┐
│                                         │
│  ⚠ Could not connect to database       │
│                                         │
│  The database file may be locked or    │
│  corrupted. Would you like to retry?   │
│                                         │
│    [R] Retry    [Q] Quit               │
└─────────────────────────────────────────┘
```

---

### Task 7.5: Performance Optimization and Polish
**Files to modify:**
- All TUI views (optimize rendering)
- `localwave-tui/Sources/App/TUIApp.swift` (startup optimization)

**Optimizations:**
- Lazy loading for large lists (virtualization)
- Debounce search input (300ms)
- Minimize state changes
- Optimize database queries (reuse from iOS)
- Cache frequently accessed data
- Reduce re-renders

**Polish items:**
- Consistent spacing and alignment
- Smooth animations where supported
- Responsive to terminal resize
- Handle edge cases (empty states, long text)
- Accessibility (screen reader support)

---

## Implementation Guidelines

### Code Reusability Strategy
1. **100% Shared**: Models, Protocols (Domain layer)
2. **100% Shared**: Repositories, Services (Data layer)
3. **100% Shared**: ViewModels (Features layer)
4. **Platform-Specific**: Views (SwiftUI vs SwiftTUI)
5. **Platform-Specific**: Audio player, File access

### SwiftTUI Limitations
- No `@Published` / `@ObservableObject` (use `@State` / `@Binding`)
- No `.foregroundColor()` modifier
- No `.environmentObject()` (pass state explicitly)
- No images (use Unicode icons/ASCII art)
- Color is background-only
- Limited to `.padding()` without parameters

### Keyboard Shortcuts Summary (Vim-Style)

**Global:**
- `1-5`: Direct tab switching
- `H/L`: Previous/Next tab (vim-style)
- `q`: Quit
- `?`: Help
- `/`: Search
- `Esc`: Back/Cancel

**Navigation (vim-style):**
- `j/k`: Down/Up (select item)
- `h/l`: Left/Right (or previous/next)
- `gg`: Go to top
- `G`: Go to bottom
- `Enter`: Open/Confirm
- `Esc`: Back

**Player:**
- `Space`: Play/Pause
- `n`: Next song
- `p`: Previous song
- `[` / `]`: Seek -10s/+10s
- `s`: Toggle Shuffle
- `r`: Cycle Repeat
- `q`: Show Queue (from player view)

**Lists (vim-style):**
- `j/k`: Navigate down/up
- `v`: Toggle selection (visual mode)
- `V`: Select all (visual line mode)
- `d`: Delete/Remove item
- `dd`: Delete selected
- `yy`: Yank/Copy

**Playlists:**
- `a`: Add songs
- `i`: Insert mode (rename)
- `n`: New playlist
- `J/K`: Reorder songs (shift+j/k)

---

## Testing Strategy

### Unit Tests
- Reuse existing ViewModel tests
- Test navigation state management
- Test keyboard input handling
- Test TUI component logic

### Integration Tests
- Database operations (shared with iOS)
- Sync/scan operations
- Queue management
- Playlist operations

### Manual TUI Testing
- Keyboard navigation flows
- Edge cases (empty lists, long text)
- Error scenarios
- Performance with large libraries (1000+ songs)

---

## Success Criteria

### Phase 2 Complete When:
- ✅ Tab bar displays with all 5 tabs
- ✅ Keyboard shortcuts (1-5) switch tabs correctly
- ✅ Navigation breadcrumb updates properly
- ✅ Reusable components (List, Table) work smoothly

### Phase 3 Complete When:
- Can browse artists, albums, songs
- Search returns FTS5 results
- Can navigate to details views
- Can edit song metadata

### Phase 4 Complete When:
- Can play songs (via platform audio player)
- Player controls respond correctly
- Queue displays and updates
- Shuffle/repeat modes work

### Phase 5 Complete When:
- Can create/delete/rename playlists
- Can add/remove songs from playlists
- Can reorder playlist songs
- Playlists persist across restarts

### Phase 6 Complete When:
- Can browse and select music folders
- Can scan folders and import songs
- Can manage sources (add/remove)
- Sync progress displays correctly

### Phase 7 Complete When:
- All features integrated seamlessly
- Help system documents all shortcuts
- Error handling covers all scenarios
- Performance is acceptable (<100ms interactions)
- 1:1 feature parity with iOS app

---

## Estimated Effort

| Phase | Tasks | Approx Lines | Time Estimate |
|-------|-------|--------------|---------------|
| 1 ✅ | 4 | ~500 | Complete |
| 2 | 4 | ~800 | 6-8 hours |
| 3 | 7 | ~1400 | 10-14 hours |
| 4 | 6 | ~1000 | 8-12 hours |
| 5 | 5 | ~900 | 7-10 hours |
| 6 | 4 | ~700 | 6-8 hours |
| 7 | 5 | ~900 | 7-10 hours |
| **Total** | **35** | **~6200** | **44-62 hours** |

---

## Next Steps

1. ✅ Complete Phase 1 (Foundation)
2. ✅ Complete Task 2.1 (Navigation system)
3. **→ Fix Task 2.2 (Keyboard input)**
4. Complete Task 2.3 (TUI components)
5. Complete Task 2.4 (Theme system)
6. Proceed with Phase 3 (Library features)

---

## Future Enhancements (Post-MVP)

- Mouse support for click interactions
- Theme customization (colors, layouts)
- ASCII visualizer for playback
- Lyrics display
- Statistics and listening history
- Batch operations (multi-edit)
- Export/import playlists (M3U, etc.)
- System media key integration
- Remote control API
- Plugin system for extensions
