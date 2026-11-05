import XCTest

final class LaunchTests: XCTestCase {
    func testQueueDragRemovalAndClearUndoPreserveTheCurrentTrack() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "now"
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["Track actions"].waitForExistence(timeout: 25))
        app.buttons["Queue"].firstMatch.tap()
        XCTAssertTrue(app.buttons["Close queue"].waitForExistence(timeout: 5))
        let parallax = app.staticTexts["Parallax"].firstMatch
        let ghost = app.staticTexts["Ghost Note"].firstMatch
        let handle = app.descendants(matching: .any).matching(identifier: "Drag to reorder Parallax").firstMatch
        XCTAssertTrue(handle.waitForExistence(timeout: 5))
        XCTAssertLessThan(parallax.frame.minY, ghost.frame.minY)
        attachScreenshot(app, name: "Queue before drag")
        handle.press(forDuration: 0.5, thenDragTo: ghost)
        let reordered = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
            ghost.exists && parallax.exists && ghost.frame.minY < parallax.frame.minY
        }, object: nil)
        XCTAssertEqual(XCTWaiter.wait(for: [reordered], timeout: 5), .completed)
        attachScreenshot(app, name: "Queue after drag")
        XCTAssertTrue(app.staticTexts["Field Day"].firstMatch.exists)
        app.buttons["Remove Ghost Note"].tap()
        let removed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: ghost)
        XCTAssertEqual(XCTWaiter.wait(for: [removed], timeout: 5), .completed)
        XCTAssertTrue(parallax.exists)
        app.buttons["Clear up next"].tap()
        XCTAssertTrue(app.staticTexts["Up next cleared."].waitForExistence(timeout: 5))
        app.buttons["Undo"].tap()
        XCTAssertTrue(parallax.waitForExistence(timeout: 5))
        XCTAssertFalse(ghost.exists, "Clear Undo must restore the queue as it was after explicit removal")
        XCTAssertTrue(app.staticTexts["Field Day"].firstMatch.exists, "Queue edits must preserve the current track")
        attachScreenshot(app, name: "Queue clear undone")
    }

    func testReducedMotionNowAndMenuActionsRemainUsable() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "lists"
        app.launchArguments = ["-lwReduceMotion", "YES"]
        app.launch()
        defer { app.terminate() }
        let list = app.staticTexts["Late drive"].firstMatch
        XCTAssertTrue(list.waitForExistence(timeout: 25))
        guard reveal(list, in: app, fullyInsideScroll: true) else { return }
        list.press(forDuration: 0.7)
        let rename = app.buttons["Rename…"]
        XCTAssertTrue(rename.waitForExistence(timeout: 5))
        attachScreenshot(app, name: "Reduced motion list menu")
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.03, dy: 0.08)).tap()
        let removed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: rename)
        XCTAssertEqual(XCTWaiter.wait(for: [removed], timeout: 5), .completed)
        let mini = app.buttons["Open now playing"].firstMatch
        XCTAssertTrue(mini.waitForExistence(timeout: 5))
        mini.tap()
        XCTAssertTrue(app.buttons["Track actions"].waitForExistence(timeout: 5))
        attachScreenshot(app, name: "Reduced motion Now")
        app.buttons["Queue"].firstMatch.tap()
        let closeQueue = app.buttons["Close queue"]
        XCTAssertTrue(closeQueue.waitForExistence(timeout: 5))
        closeQueue.tap()
        app.buttons["Close now playing"].tap()
        XCTAssertTrue(mini.waitForExistence(timeout: 5))
        attachScreenshot(app, name: "Reduced motion mini restored")
    }

    func testReferenceFreshArtistsSongsAndDetails() throws {
        for state in ["songs", "artists", "album", "artist", "list", "favorites", "find-results"] {
            let app = XCUIApplication()
            app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
            app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
            app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = state
            app.launch()
            switch state {
            case "songs": XCTAssertTrue(app.buttons["Actions for Antenna"].waitForExistence(timeout: 25))
            case "artists":
                XCTAssertTrue(app.buttons["library-mode-Artists"].waitForExistence(timeout: 25))
                XCTAssertTrue(app.staticTexts["Halden Marsh"].firstMatch.waitForExistence(timeout: 10))
            case "album":
                XCTAssertTrue(app.buttons["library-play-context"].waitForExistence(timeout: 25))
                XCTAssertTrue(app.staticTexts["Field Recordings 04"].firstMatch.waitForExistence(timeout: 10))
            case "artist": XCTAssertTrue(app.buttons["library-play-context"].waitForExistence(timeout: 25))
            case "list": XCTAssertTrue(app.staticTexts["Late drive"].firstMatch.waitForExistence(timeout: 25))
            case "favorites": XCTAssertTrue(app.buttons["Actions for Harbour"].waitForExistence(timeout: 25))
            default:
                XCTAssertTrue(app.textFields["Songs, albums, folders"].waitForExistence(timeout: 25))
                XCTAssertTrue(app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "Actions for ")).firstMatch.waitForExistence(timeout: 10))
            }
            Thread.sleep(forTimeInterval: 1)
            if state == "list" || state == "favorites" {
                XCTAssertTrue(app.buttons["Lists"].firstMatch.isSelected, "List details retain the selected Lists navigation cell")
            }
            attachScreenshot(app, name: "Reference fresh " + state)
            app.terminate()
        }
    }

    func testReferenceVisualNowAndSheets() throws {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launch()
        let mini = app.buttons["Open now playing"].firstMatch
        XCTAssertTrue(mini.waitForExistence(timeout: 25))
        XCTAssertTrue(app.staticTexts["Field Day"].firstMatch.waitForExistence(timeout: 25), "The reference track must populate the mini before testing its artwork transition")
        mini.tap()
        XCTAssertTrue(app.buttons["Track actions"].waitForExistence(timeout: 25))
        Thread.sleep(forTimeInterval: 1)
        attachScreenshot(app, name: "Reference Now")
        app.buttons["Queue"].firstMatch.tap()
        XCTAssertTrue(app.buttons["Close queue"].waitForExistence(timeout: 5))
        Thread.sleep(forTimeInterval: 1)
        attachScreenshot(app, name: "Reference Queue")
        app.buttons["Close queue"].tap()
        Thread.sleep(forTimeInterval: 0.4)
        app.buttons["Track actions"].tap()
        XCTAssertTrue(app.buttons["Play next"].waitForExistence(timeout: 5))
        Thread.sleep(forTimeInterval: 1)
        attachScreenshot(app, name: "Reference Track actions")
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.03, dy: 0.08)).tap()
        Thread.sleep(forTimeInterval: 0.4)
        app.buttons["Close now playing"].tap()
        XCTAssertTrue(mini.waitForExistence(timeout: 5))
        Thread.sleep(forTimeInterval: 0.4)
        attachScreenshot(app, name: "Reference mini after close")
    }

    func testNowTrackMenuSubmenuCreatesListWithThePlayingSong() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "now"
        app.launch()
        defer { app.terminate() }
        let more = app.buttons["Track actions"]
        XCTAssertTrue(more.waitForExistence(timeout: 25))
        more.tap()
        let add = app.buttons["Add to list"]
        XCTAssertTrue(add.waitForExistence(timeout: 5), "Now must expose the shared track menu, including its list submenu")
        XCTAssertTrue(app.buttons["Share…"].exists)
        Thread.sleep(forTimeInterval: 0.7)
        app.staticTexts["Field Day"].firstMatch.tap()
        XCTAssertTrue(add.exists, "Tapping the menu preview must keep its actions open")
        let playCandidates = app.buttons.matching(identifier: "Play").allElementsBoundByIndex
        let diagnostic = XCTAttachment(string: playCandidates.map { "frame=\($0.frame) hittable=\($0.isHittable)" }.joined(separator: "\n"))
        diagnostic.name = "Menu Play hit regions"
        diagnostic.lifetime = .keepAlways
        self.add(diagnostic)
        guard let menuPlay = playCandidates.first(where: { $0.isHittable && $0.frame.width > 100 }) else {
            XCTFail("The full-width menu Play row must be hittable")
            return
        }
        let playBounds = menuPlay.frame
        app.coordinate(withNormalizedOffset: .zero).withOffset(CGVector(dx: playBounds.midX, dy: playBounds.minY - 5)).tap()
        XCTAssertTrue(add.exists, "The gap inside the menu must not activate the backdrop")
        attachScreenshot(app, name: "Now shared track menu")
        add.tap()
        let back = app.buttons["‹ Add to list"]
        XCTAssertTrue(back.waitForExistence(timeout: 5))
        back.tap()
        XCTAssertTrue(add.waitForExistence(timeout: 5))
        add.tap()
        app.buttons["New list…"].tap()
        let name = app.textFields["List name"]
        XCTAssertTrue(name.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["1 song will be added."].exists)
        name.typeText("Now track list")
        app.buttons["Create"].tap()
        let closed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: name)
        XCTAssertEqual(XCTWaiter.wait(for: [closed], timeout: 5), .completed)
        app.buttons["Close now playing"].tap()
        app.buttons["Lists"].firstMatch.tap()
        let created = app.staticTexts["Now track list"].firstMatch
        guard reveal(created, in: app) else { return }
        created.tap()
        XCTAssertTrue(app.buttons["Actions for Field Day"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["1 songs"].exists, "The created list must contain the actual playing song exactly once")
    }

    func testNowPlaylistMenuRemovesAndUndoesTheActualMembership() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["library-mode-Songs"].waitForExistence(timeout: 20))
        app.buttons["library-mode-Songs"].tap()
        let actions = app.buttons["Actions for Playlist Test Track"]
        XCTAssertTrue(actions.waitForExistence(timeout: 20))
        actions.tap()
        app.buttons["Add to list"].tap()
        app.buttons["UI Test Playlist"].tap()
        app.buttons["Lists"].firstMatch.tap()
        let list = app.staticTexts["UI Test Playlist"].firstMatch
        guard reveal(list, in: app) else { return }
        list.tap()
        let play = app.buttons["library-play-context"]
        XCTAssertTrue(play.waitForExistence(timeout: 5))
        play.tap()
        XCTAssertTrue(app.buttons["Pause"].firstMatch.waitForExistence(timeout: 10))
        app.buttons["Open now playing"].firstMatch.tap()
        let more = app.buttons["Track actions"]
        XCTAssertTrue(more.waitForExistence(timeout: 5))
        more.tap()
        XCTAssertTrue(app.buttons["Remove from list"].waitForExistence(timeout: 5))
        app.buttons["Add to list"].tap()
        XCTAssertTrue(app.buttons["New list…"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["UI Test Playlist"].exists, "The current list must be excluded from Add to list")
        app.buttons["‹ Add to list"].tap()
        app.buttons["Remove from list"].tap()
        let undo = app.buttons["Undo"]
        XCTAssertTrue(undo.waitForExistence(timeout: 5), "Undo must be available while Now is open")
        XCTAssertTrue(app.buttons["Close now playing"].exists)
        undo.tap()
        XCTAssertTrue(app.staticTexts["Undone"].waitForExistence(timeout: 5))
        app.buttons["Close now playing"].tap()
        XCTAssertTrue(actions.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["1 songs"].exists, "Undo must restore the real playlist membership")
    }

    func testReferenceVisualLibraryListsAndFind() throws {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launch()
        let count = app.descendants(matching: .any).matching(identifier: "library-song-count").firstMatch
        XCTAssertTrue(app.buttons["library-mode-Albums"].waitForExistence(timeout: 25))
        XCTAssertTrue(app.staticTexts["Field Day"].firstMatch.waitForExistence(timeout: 15))
        XCTAssertEqual(count.value as? String, "92")
        let layout = XCTAttachment(string: "count=\(count.value ?? "nil") scrollFrames=\(app.scrollViews.allElementsBoundByIndex.map(\.frame)) albumFrames=\(app.staticTexts.matching(identifier: "Field Recordings 04").allElementsBoundByIndex.map(\.frame))")
        layout.name = "Reference Library layout frames"
        layout.lifetime = .keepAlways
        add(layout)
        Thread.sleep(forTimeInterval: 1)
        attachScreenshot(app, name: "Reference Library albums")
        app.buttons["library-mode-Songs"].tap()
        XCTAssertTrue(app.buttons["Actions for Antenna"].waitForExistence(timeout: 5), "Reference Songs mode must be visible before its screenshot")
        Thread.sleep(forTimeInterval: 1)
        attachScreenshot(app, name: "Reference Library songs")
        app.buttons["Lists"].firstMatch.tap()
        XCTAssertTrue(app.staticTexts["Automatic"].waitForExistence(timeout: 5))
        Thread.sleep(forTimeInterval: 1)
        attachScreenshot(app, name: "Reference Lists")
        app.buttons["Find"].firstMatch.tap()
        XCTAssertTrue(app.textFields["Songs, albums, folders"].waitForExistence(timeout: 5))
        Thread.sleep(forTimeInterval: 1)
        attachScreenshot(app, name: "Reference Find")
    }

    func testRootTabsRetainLibraryModeAndFindQuery() throws {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launch()
        let songsMode = app.buttons["library-mode-Songs"]
        XCTAssertTrue(songsMode.waitForExistence(timeout: 25))
        guard reveal(songsMode, in: app) else { return }
        songsMode.tap()
        XCTAssertTrue(app.buttons["Actions for Antenna"].waitForExistence(timeout: 15))
        app.buttons["Lists"].firstMatch.tap()
        XCTAssertTrue(app.staticTexts["Automatic"].waitForExistence(timeout: 10))
        app.buttons["Library"].firstMatch.tap()
        XCTAssertTrue(songsMode.waitForExistence(timeout: 10))
        XCTAssertTrue(songsMode.isSelected, "Returning to Library keeps the selected Songs mode")
        XCTAssertTrue(app.buttons["Actions for Antenna"].waitForExistence(timeout: 15))

        app.buttons["Find"].firstMatch.tap()
        let query = app.textFields["Songs, albums, folders"]
        XCTAssertTrue(query.waitForExistence(timeout: 10))
        app.buttons["cloud"].firstMatch.tap()
        XCTAssertEqual(query.value as? String, "cloud")
        XCTAssertTrue(app.buttons["Actions for Harbour"].waitForExistence(timeout: 15))
        app.buttons["Lists"].firstMatch.tap()
        XCTAssertTrue(app.staticTexts["Automatic"].waitForExistence(timeout: 10))
        app.buttons["Find"].firstMatch.tap()
        XCTAssertTrue(query.waitForExistence(timeout: 10))
        XCTAssertEqual(query.value as? String, "cloud", "Returning to Find retains its actual query")
        XCTAssertTrue(app.buttons["Actions for Harbour"].waitForExistence(timeout: 15))
        attachScreenshot(app, name: "Retained Find cloud query")
    }

    func testAdaptiveLibraryListsAndFind() throws {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launch()
        let mobileSongs = app.buttons["library-mode-Songs"]
        let songs = mobileSongs.waitForExistence(timeout: 2) ? mobileSongs : app.buttons["Songs"].firstMatch
        XCTAssertTrue(songs.waitForExistence(timeout: 15))
        guard reveal(songs, in: app) else { return }
        songs.tap()
        let trackActions = app.buttons["Actions for Playlist Test Track"]
        guard reveal(trackActions, in: app) else { return }
        XCTAssertTrue(trackActions.waitForExistence(timeout: 15))
        attachScreenshot(app, name: "Adaptive library")
        trackActions.tap()
        XCTAssertTrue(app.buttons["Add to list"].waitForExistence(timeout: 5))
        attachScreenshot(app, name: "Adaptive track menu with system safe areas")
        app.buttons["Add to list"].tap()
        let playlistChoice = try XCTUnwrap(app.buttons.matching(identifier: "UI Test Playlist").allElementsBoundByIndex.last)
        playlistChoice.tap()

        app.buttons["Lists"].firstMatch.tap()
        let seededList = app.staticTexts["UI Test Playlist"]
        guard reveal(seededList, in: app) else { return }
        XCTAssertTrue(seededList.waitForExistence(timeout: 10))
        seededList.tap()
        guard reveal(app.staticTexts["Playlist Test Track"], in: app) else { return }
        XCTAssertTrue(app.staticTexts["Playlist Test Track"].waitForExistence(timeout: 10))
        attachScreenshot(app, name: "List detail")
        app.buttons["Back"].firstMatch.tap()
        let returnedToLists = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: app.buttons["Back"].firstMatch)
        XCTAssertEqual(XCTWaiter.wait(for: [returnedToLists], timeout: 5), .completed)

        guard reveal(app.buttons["New list"].firstMatch, in: app, fullyInsideScroll: true) else { return }
        app.buttons["New list"].firstMatch.tap()
        let listName = app.textFields["List name"]
        XCTAssertTrue(listName.waitForExistence(timeout: 5))
        listName.typeText("Evening music")
        app.buttons["Create"].tap()
        guard reveal(app.staticTexts["Evening music"], in: app) else { return }
        XCTAssertTrue(app.staticTexts["Evening music"].waitForExistence(timeout: 10))

        // Find is a deck item on phones and a keyboard command on wide layouts.
        let find = app.buttons["Find"]
        if find.exists && find.isHittable { find.tap() }
        else {
            app.buttons["Search"].tap()
            let commands = app.textFields["Search commands…"]
            XCTAssertTrue(commands.waitForExistence(timeout: 5))
            commands.tap()
            commands.typeText("Find")
            app.buttons["Find"].firstMatch.tap()
        }
        let search = app.textFields["Songs, albums, folders"]
        XCTAssertTrue(search.waitForExistence(timeout: 10))
        search.tap()
        search.typeText("Playlist Test")
        guard reveal(app.buttons["Actions for Playlist Test Track"], in: app) else { return }
        XCTAssertTrue(app.buttons["Actions for Playlist Test Track"].waitForExistence(timeout: 10))
        attachScreenshot(app, name: "Find results")

        app.buttons["Folders"].firstMatch.tap()
        let keyboardDismissed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: app.keyboards.firstMatch)
        XCTAssertEqual(XCTWaiter.wait(for: [keyboardDismissed], timeout: 5), .completed)
        guard reveal(app.staticTexts["Point it at a folder."], in: app) else { return }
        XCTAssertTrue(app.staticTexts["Point it at a folder."].waitForExistence(timeout: 10))
        guard reveal(app.buttons["Add music"].firstMatch, in: app) else { return }
        app.buttons["Add music"].firstMatch.tap()
        XCTAssertTrue(app.buttons.containing(.staticText, identifier: "A folder").firstMatch.waitForExistence(timeout: 5))
        attachScreenshot(app, name: "Add music")
    }

    func testPlaybackWaveformAndQueue() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launch()
        let mobileSongs = app.buttons["library-mode-Songs"]
        let songs = mobileSongs.waitForExistence(timeout: 3) ? mobileSongs : app.buttons["Songs"].firstMatch
        XCTAssertTrue(songs.waitForExistence(timeout: 15))
        guard reveal(songs, in: app) else { return }
        songs.tap()
        let track = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "play-song-")).firstMatch
        guard reveal(track, in: app) else { return }
        XCTAssertTrue(track.waitForExistence(timeout: 15))
        if mobileSongs.exists { track.tap() } else { track.doubleTap() }
        XCTAssertTrue(app.buttons["Pause"].firstMatch.waitForExistence(timeout: 15))
        let expand = app.buttons["Open now playing"]
        if expand.exists { expand.firstMatch.tap() }
        let queue = app.buttons["Queue"].firstMatch
        guard reveal(queue, in: app) else { return }
        XCTAssertTrue(queue.waitForExistence(timeout: 10))
        attachScreenshot(app, name: "Now playing with decoded waveform")
        let waveform = app.otherElements["Playback position"].firstMatch
        if waveform.exists {
            guard reveal(waveform, in: app, fullyInsideScroll: true) else { return }
            let start = waveform.coordinate(withNormalizedOffset: CGVector(dx: 0.15, dy: 0.5))
            let end = waveform.coordinate(withNormalizedOffset: CGVector(dx: 0.6, dy: 0.5))
            start.press(forDuration: 0.05, thenDragTo: end)
        }
        guard reveal(queue, in: app) else { return }
        queue.tap()
        XCTAssertTrue(app.buttons["Close queue"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Playlist Test Track"].firstMatch.waitForExistence(timeout: 5))
        attachScreenshot(app, name: "Playback queue")
        app.buttons["Close queue"].tap()
        let dismissed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: app.buttons["Close queue"])
        XCTAssertEqual(XCTWaiter.wait(for: [dismissed], timeout: 5), .completed)
        let playback = app.buttons["player.playPause"].firstMatch
        let visible = XCTNSPredicateExpectation(predicate: NSPredicate(format: "hittable == true"), object: playback)
        XCTAssertEqual(XCTWaiter.wait(for: [visible], timeout: 5), .completed)
        playback.tap()
        let paused = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label == %@", "Play"), object: playback)
        XCTAssertEqual(XCTWaiter.wait(for: [paused], timeout: 5), .completed)
    }

    func testNowPlayingContextReturnsToAlbum() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launch()
        let album = app.staticTexts["Test Album"].firstMatch
        guard reveal(album, in: app) else { return }
        XCTAssertTrue(album.waitForExistence(timeout: 15))
        album.tap()
        let play = app.buttons["library-play-context"]
        guard reveal(play, in: app) else { return }
        XCTAssertTrue(play.waitForExistence(timeout: 10))
        play.tap()
        XCTAssertTrue(app.buttons["Pause"].firstMatch.waitForExistence(timeout: 15))
        let expand = app.buttons["Open now playing"]
        if expand.exists { expand.firstMatch.tap() }
        let context = app.buttons["Playing from Test Album"]
        if context.waitForExistence(timeout: 3) { context.tap() }
        else {
            app.buttons["Track actions"].tap()
            app.buttons["Show playing source"].tap()
        }
        let dismissed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: app.buttons["Close now playing"])
        XCTAssertEqual(XCTWaiter.wait(for: [dismissed], timeout: 5), .completed)
        guard reveal(app.buttons["library-play-context"], in: app) else { return }
        XCTAssertTrue(app.buttons["library-play-context"].waitForExistence(timeout: 10))
        attachScreenshot(app, name: "Playback context returns to album")
        if app.buttons["Pause"].firstMatch.exists { app.buttons["Pause"].firstMatch.tap() }
    }

    private func attachScreenshot(_ app: XCUIApplication, name: String) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
