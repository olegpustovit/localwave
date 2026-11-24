This build is the Rust LocalWave on the same library and settings as before. Your songs, queue, playlists and settings should all be where you left them.

What changed:
- Much lighter on battery and GPU. LocalWave now redraws only what changes. During playback with the song list open it uses about 92% less GPU, an idle or hidden window draws nothing, and the playing indicator animates smoothly at 20 frames per second.
- Song numbers in the list stay on one line instead of wrapping.

Please check:
- Your existing library, queue, favorites and playlists open correctly after updating.
- Scrolling long lists, resizing the window and switching light/dark look right with no flicker or stale pixels.
- Playback controls, the clock and the playing indicator behave normally, including after a song repeats.
- On a Mac, LocalWave uses little CPU or battery when idle or in the background.
