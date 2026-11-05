import XCTest

final class FolderActionTests: XCTestCase {
    func testCompactFolderSelectAllDoneAndReentryUseTheSelectionDeck() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_UI_SOURCE"] = "1"
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["Folders"].firstMatch.waitForExistence(timeout: 20))
        app.buttons["Folders"].firstMatch.tap()
        let source = app.buttons.containing(.staticText, identifier: "UI Test Source").firstMatch
        XCTAssertTrue(source.waitForExistence(timeout: 45))
        guard reveal(source, in: app) else { return }
        source.tap()
        let select = app.buttons["folder.select"]
        XCTAssertTrue(select.waitForExistence(timeout: 15))
        select.tap()
        XCTAssertTrue(app.staticTexts["0 selected"].waitForExistence(timeout: 5))
        attach(app, name: "Folder selection zero selected")
        for title in ["Play next", "Add to list", "Offline", "Queue"] {
            XCTAssertTrue(app.buttons[title].exists)
            XCTAssertFalse(app.buttons[title].isEnabled)
            XCTAssertEqual(app.buttons[title].frame.height, 62, accuracy: 1)
        }
        XCTAssertFalse(app.buttons["Shuffle everything"].exists)
        XCTAssertFalse(app.buttons["Find"].exists, "The normal navigation deck must be replaced during folder selection")
        app.buttons["Select all"].tap()
        XCTAssertTrue(app.staticTexts["1 selected"].waitForExistence(timeout: 5))
        attach(app, name: "Folder selection all saved songs selected")
        XCTAssertTrue(app.buttons["Queue"].isEnabled)
        XCTAssertTrue(app.buttons["Deselect all"].exists)
        app.buttons["Done"].tap()
        XCTAssertTrue(app.buttons["Find"].waitForExistence(timeout: 5))
        XCTAssertTrue(select.waitForExistence(timeout: 5))
        select.tap()
        XCTAssertTrue(app.staticTexts["0 selected"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Queue"].isEnabled)
        app.buttons["Done"].tap()
    }

    func testRealSourcePlaybackAvailabilityQueueAndFavorite() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_UI_SOURCE"] = "1"
        app.launch()
        defer { app.terminate() }
        let folders = app.buttons["Folders"].firstMatch
        XCTAssertTrue(folders.waitForExistence(timeout: 15))
        folders.tap()
        let source = app.staticTexts["UI Test Source"].firstMatch
        XCTAssertTrue(source.waitForExistence(timeout: 45))
        guard reveal(source, in: app) else { return }
        source.tap()
        let play = app.buttons["Play folder"].firstMatch
        guard reveal(play, in: app) else { return }
        XCTAssertTrue(play.waitForExistence(timeout: 15))
        let enabled = XCTNSPredicateExpectation(predicate: NSPredicate(format: "enabled == true"), object: play)
        XCTAssertEqual(XCTWaiter.wait(for: [enabled], timeout: 15), .completed)
        play.tap()
        let pause = app.buttons["Pause"].firstMatch
        XCTAssertTrue(pause.waitForExistence(timeout: 15))
        pause.tap()

        let offline = app.buttons["Offline 1"].firstMatch
        guard reveal(offline, in: app) else { return }
        XCTAssertTrue(offline.waitForExistence(timeout: 15))
        offline.tap()
        let actions = app.buttons["Actions for Folder Test Track.wav"]
        guard reveal(actions, in: app) else { return }
        XCTAssertTrue(actions.waitForExistence(timeout: 10))
        actions.tap()
        app.buttons["Add to queue"].tap()
        waitForDismissal(app.buttons["Add to queue"])
        actions.tap()
        app.buttons["Favorite"].tap()
        waitForDismissal(app.buttons["Favorite"])
        // Reopening proves the menu now reflects the persisted repository favorite.
        let favoriteChanged = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == %@", "Favorite"), object: actions)
        XCTAssertEqual(XCTWaiter.wait(for: [favoriteChanged], timeout: 10), .completed)
        actions.tap()
        XCTAssertTrue(app.buttons["Unfavorite"].waitForExistence(timeout: 10))
        app.buttons["Unfavorite"].tap()
        waitForDismissal(app.buttons["Unfavorite"])
        attach(app, name: "Real source with offline availability and native actions")

        let expand = app.buttons["Open now playing"].firstMatch
        if expand.exists { expand.tap() }
        let queue = app.buttons["Queue"].firstMatch
        guard reveal(queue, in: app) else { return }
        XCTAssertTrue(queue.waitForExistence(timeout: 10))
        queue.tap()
        XCTAssertTrue(app.buttons["Close queue"].waitForExistence(timeout: 10))
        let rows = app.staticTexts.matching(identifier: "Folder Test Track.wav")
        let duplicated = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in rows.count >= 2 }, object: nil)
        XCTAssertEqual(XCTWaiter.wait(for: [duplicated], timeout: 10), .completed)
        attach(app, name: "Source row appended to real playback queue")
        app.buttons["Close queue"].tap()
    }

    func testNowFolderJumpUsesFilesAncestryAndRetainsFindQuery() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_UI_SOURCE"] = "1"
        app.launch()
        defer { app.terminate() }
        let songs = app.buttons["library-mode-Songs"]
        XCTAssertTrue(songs.waitForExistence(timeout: 20))
        songs.tap()
        let track = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@ AND label CONTAINS %@", "play-song-", "Folder Test Track.wav")).firstMatch
        XCTAssertTrue(track.waitForExistence(timeout: 45))
        guard reveal(track, in: app) else { return }
        track.tap()
        let pause = app.buttons["Pause"].firstMatch
        XCTAssertTrue(pause.waitForExistence(timeout: 15))
        pause.tap()
        app.buttons["Find"].firstMatch.tap()
        let query = app.textFields["Songs, albums, folders"]
        XCTAssertTrue(query.waitForExistence(timeout: 5))
        query.tap(); query.typeText("Folder")
        XCTAssertTrue(app.buttons["Actions for Folder Test Track.wav"].waitForExistence(timeout: 10))
        let expand = app.buttons["Open now playing"].firstMatch
        XCTAssertTrue(expand.waitForExistence(timeout: 5))
        expand.tap()
        let actions = app.buttons["Track actions"]
        XCTAssertTrue(actions.waitForExistence(timeout: 5))
        let keyboardDismissed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: app.keyboards.firstMatch)
        XCTAssertEqual(XCTWaiter.wait(for: [keyboardDismissed], timeout: 5), .completed)
        actions.tap()
        let folder = app.buttons["Show in folder"]
        XCTAssertTrue(folder.waitForExistence(timeout: 5))
        folder.tap()
        let back = app.buttons["folder.back"]
        XCTAssertTrue(back.waitForExistence(timeout: 10))
        XCTAssertTrue(back.label.contains("Folders"), "The final reference Show in folder action starts Files ancestry")
        XCTAssertTrue(app.staticTexts["folder.title"].waitForExistence(timeout: 5))
        back.tap()
        XCTAssertTrue(app.buttons["Add music"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["folder.back"].exists, "Back at the selected source root returns to Files")
        app.buttons["Find"].firstMatch.tap()
        XCTAssertTrue(query.waitForExistence(timeout: 5))
        XCTAssertEqual(query.value as? String, "Folder")
        XCTAssertTrue(app.buttons["Actions for Folder Test Track.wav"].exists)
    }

    func testFilesVisualGeometry() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_UI_SOURCE"] = "1"
        app.launch()
        defer { app.terminate() }
        let folders = app.buttons["Folders"].firstMatch
        XCTAssertTrue(folders.waitForExistence(timeout: 15))
        folders.tap()
        XCTAssertTrue(app.staticTexts["UI Test Source"].firstMatch.waitForExistence(timeout: 45))
        attach(app, name: "Files canonical normal light")
        let sourceCard = app.buttons.containing(.staticText, identifier: "UI Test Source").firstMatch
        XCTAssertTrue(sourceCard.waitForExistence(timeout: 15))
        sourceCard.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.85)).tap()
        XCTAssertTrue(app.buttons["Play folder"].firstMatch.waitForExistence(timeout: 10))
        let headerBack = app.buttons["folder.back"]
        XCTAssertEqual(app.buttons.matching(identifier: "folder.back").count, 1)
        XCTAssertEqual(headerBack.label, "Folders")
        XCTAssertLessThan(headerBack.frame.midY, app.frame.midY)
        headerBack.tap()
        XCTAssertFalse(app.buttons["folder.back"].exists)
        let add = app.buttons["Add music"].firstMatch
        XCTAssertTrue(add.waitForExistence(timeout: 10))
        add.tap()
        XCTAssertTrue(app.staticTexts["A folder"].firstMatch.waitForExistence(timeout: 10))
        attach(app, name: "Add music canonical normal light")
        app.buttons["Close"].firstMatch.tap()
        app.buttons["Library"].firstMatch.tap()
        let settings = app.buttons["Settings"].firstMatch
        XCTAssertTrue(settings.waitForExistence(timeout: 10))
        settings.tap()
        XCTAssertTrue(app.staticTexts["Appearance"].firstMatch.waitForExistence(timeout: 10))
        attach(app, name: "Settings canonical normal light")
    }

    func testReferenceSourcesAddAndSettingsGeometry() {
        let app = XCUIApplication()
        for screen in ["folders", "add", "settings"] {
            app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
            app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
            app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = screen
            app.launch()
            switch screen {
            case "folders":
                XCTAssertTrue(app.staticTexts["iCloud Drive"].firstMatch.waitForExistence(timeout: 20))
                attach(app, name: "Reference sources normal light")
                let reconnect = app.buttons["Reconnect folder"].firstMatch
                guard reveal(reconnect, in: app) else { app.terminate(); return }
                attach(app, name: "Reference sources reconnect state")
            case "add":
                XCTAssertTrue(app.staticTexts["A folder"].waitForExistence(timeout: 20))
                attach(app, name: "Reference Add music")
            default:
                XCTAssertTrue(app.switches["Download ahead"].waitForExistence(timeout: 20))
                attach(app, name: "Reference Settings")
            }
            app.terminate()
        }
    }

    func testOwnedSourceChildAndBackRoutesShowActualFolderRows() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "folders"
        app.launch()
        defer { app.terminate() }
        let source = app.buttons["source.open.1"]
        XCTAssertTrue(source.waitForExistence(timeout: 25))
        source.tap()
        let title = app.staticTexts["folder.title"]
        let sourceShown = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == true AND label == %@", "iCloud Drive"), object: title)
        XCTAssertEqual(XCTWaiter.wait(for: [sourceShown], timeout: 15), .completed)
        let child = app.buttons["folder.directory.Halden Marsh"]
        XCTAssertTrue(child.waitForExistence(timeout: 15))
        guard reveal(child, in: app) else { return }
        child.tap()
        let childShown = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == true AND label == %@", "Halden Marsh"), object: title)
        XCTAssertEqual(XCTWaiter.wait(for: [childShown], timeout: 15), .completed)
        XCTAssertTrue(app.buttons["folder.directory.Field Recordings 04"].waitForExistence(timeout: 10))
        XCTAssertFalse(app.buttons["folder.directory.Halden Marsh"].exists)
        attach(app, name: "Owned source route into actual child folder")
        app.buttons["folder.back"].tap()
        XCTAssertEqual(XCTWaiter.wait(for: [XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == true AND label == %@", "iCloud Drive"), object: title)], timeout: 15), .completed)
        XCTAssertTrue(child.waitForExistence(timeout: 10))
        app.buttons["folder.back"].tap()
        XCTAssertTrue(app.buttons["Add music"].waitForExistence(timeout: 10))
        XCTAssertFalse(app.buttons["folder.back"].exists)
        XCTAssertTrue(source.exists)
        attach(app, name: "Owned source Back restores Folders root")
    }

    func testFolderOptionsCustomSheetAndRemovalCancelKeepSource() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_UI_SOURCE"] = "1"
        app.launch()
        defer { app.terminate() }
        let folders = app.buttons["Folders"].firstMatch
        XCTAssertTrue(folders.waitForExistence(timeout: 20))
        folders.tap()
        let source = app.buttons.containing(.staticText, identifier: "UI Test Source").firstMatch
        XCTAssertTrue(source.waitForExistence(timeout: 45))
        guard reveal(source, in: app) else { return }
        source.tap()
        let options = app.buttons["Folder options"]
        XCTAssertTrue(options.waitForExistence(timeout: 15))
        options.tap()
        XCTAssertTrue(app.buttons["Keep this folder offline"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Add folder to queue"].exists)
        attach(app, name: "Custom source folder options sheet")
        app.buttons["Remove source (files are not deleted)"].tap()
        let title = app.staticTexts["Remove UI Test Source?"]
        XCTAssertTrue(title.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Its songs leave the library. Files are not deleted."].exists)
        attach(app, name: "Custom source removal confirmation")
        app.buttons["Cancel"].tap()
        XCTAssertTrue(app.buttons["folder.audio.Folder Test Track.wav"].waitForExistence(timeout: 10))
        app.buttons["folder.back"].tap()
        XCTAssertTrue(source.waitForExistence(timeout: 10))
    }

    private func waitForDismissal(_ item: XCUIElement) {
        let dismissed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: item)
        XCTAssertEqual(XCTWaiter.wait(for: [dismissed], timeout: 5), .completed)
    }

    private func attach(_ app: XCUIApplication, name: String) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
