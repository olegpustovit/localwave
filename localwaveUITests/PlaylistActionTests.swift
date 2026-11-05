import XCTest

final class PlaylistActionTests: XCTestCase {
    func testCompactListSwipeRemovalCanBeUndone() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "list"
        app.launch()
        defer { app.terminate() }
        let row = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "play-song-"))
            .containing(.staticText, identifier: "Low Tide").firstMatch
        XCTAssertTrue(row.waitForExistence(timeout: 25))
        let start = row.coordinate(withNormalizedOffset: CGVector(dx: 0.65, dy: 0.5))
        attach(app, "List before custom swipe")
        start.press(forDuration: 0.05, thenDragTo: start.withOffset(CGVector(dx: -120, dy: 0)))
        XCTAssertTrue(app.staticTexts["7 songs"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Field Day"].exists, "A swipe must not activate the row that moves underneath the released finger")
        let undo = app.buttons["Undo"]
        guard undo.exists else { XCTFail("Undo must be available immediately after the swipe"); return }
        undo.tap()
        XCTAssertTrue(app.staticTexts["8 songs"].waitForExistence(timeout: 5))
        XCTAssertTrue(row.exists)
        attach(app, "Custom swipe removal undone")
        app.buttons["Edit"].tap()
        row.tap()
        XCTAssertTrue(app.staticTexts["1 selected"].waitForExistence(timeout: 5), "The next intentional row tap must still perform its action")
    }

    func testCompactSelectionDeckRemovesAndUndoesWithoutRestoringSelection() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "list"
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["Edit"].waitForExistence(timeout: 25))
        app.buttons["Edit"].tap()
        XCTAssertTrue(app.staticTexts["0 selected"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Select all"].exists)
        XCTAssertFalse(app.buttons["Remove"].isEnabled)
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "play-song-"))
        let tide = rows.containing(.staticText, identifier: "Low Tide").firstMatch
        let parallax = rows.containing(.staticText, identifier: "Parallax").firstMatch
        XCTAssertTrue(tide.waitForExistence(timeout: 5))
        tide.tap()
        parallax.tap()
        XCTAssertTrue(app.staticTexts["2 selected"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Play next"].exists)
        XCTAssertTrue(app.buttons["Add to list"].exists)
        XCTAssertTrue(app.buttons["Offline"].exists)
        XCTAssertFalse(app.buttons["Open now playing"].exists, "Selection replaces the normal mini player and navigation deck")
        attach(app, "Compact two-song selection")
        tide.press(forDuration: 0.7)
        XCTAssertFalse(app.buttons["Share…"].exists, "Selection rows must not open a track context menu")
        XCTAssertTrue(app.staticTexts["1 selected"].exists, "Releasing a selection row still performs its ordinary selection action")
        tide.tap()
        XCTAssertTrue(app.staticTexts["2 selected"].waitForExistence(timeout: 5))
        let handle = app.descendants(matching: .any).matching(NSPredicate(format: "label == %@", "Reorder Low Tide")).firstMatch
        guard handle.waitForExistence(timeout: 5) else { XCTFail("The reference reorder handle must remain accessible"); return }
        handle.press(forDuration: 0.5, thenDragTo: parallax)
        let reordered = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
            parallax.exists && tide.exists && parallax.frame.minY < tide.frame.minY
        }, object: nil)
        XCTAssertEqual(XCTWaiter.wait(for: [reordered], timeout: 5), .completed)
        XCTAssertTrue(app.staticTexts["2 selected"].exists, "Reordering must preserve membership and selection")
        attach(app, "Compact selection after custom-handle reorder")
        app.buttons["Remove"].tap()
        XCTAssertTrue(app.staticTexts["6 songs"].waitForExistence(timeout: 5))
        let undo = app.buttons["Undo"]
        XCTAssertTrue(undo.waitForExistence(timeout: 3))
        undo.tap()
        XCTAssertTrue(app.staticTexts["8 songs"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Edit"].exists)
        XCTAssertTrue(app.buttons["Open now playing"].exists)
        XCTAssertFalse(app.staticTexts["2 selected"].exists)
        XCTAssertFalse(app.buttons["Remove"].exists, "Undo restores membership without reopening selection")
        XCTAssertLessThan(parallax.frame.minY, tide.frame.minY, "Undo must restore the order captured after the drag")
        attach(app, "Compact selection removal undone")
    }

    func testContextRenameDuplicateAndConfirmedDeletePreserveOriginal() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "lists"
        app.launch()
        let original = app.staticTexts["Late drive"].firstMatch
        XCTAssertTrue(original.waitForExistence(timeout: 25))
        guard reveal(original, in: app) else { return }
        original.press(forDuration: 1)
        let rename = app.buttons["Rename…"]
        XCTAssertTrue(rename.waitForExistence(timeout: 5))
        attach(app, "List context menu")
        rename.tap()
        let field = app.textFields["List name"]
        XCTAssertTrue(field.waitForExistence(timeout: 5))
        XCTAssertEqual(field.value as? String, "Late drive")
        attach(app, "Rename List dialog")
        field.typeText("Renamed list")
        XCTAssertEqual(field.value as? String, "Renamed list", "Rename must focus and select the existing name")
        app.buttons["Save"].tap()
        let renamed = app.staticTexts["Renamed list"].firstMatch
        XCTAssertTrue(renamed.waitForExistence(timeout: 5))
        guard reveal(renamed, in: app) else { return }
        renamed.press(forDuration: 1)
        app.buttons["Duplicate"].tap()
        let copy = app.staticTexts["Renamed list copy"].firstMatch
        XCTAssertTrue(copy.waitForExistence(timeout: 5))
        guard reveal(copy, in: app) else { return }
        copy.press(forDuration: 1)
        app.buttons["Delete list"].tap()
        XCTAssertTrue(app.buttons["Delete List"].waitForExistence(timeout: 5))
        attach(app, "Delete List confirmation")
        app.buttons["Cancel"].tap()
        XCTAssertTrue(copy.waitForExistence(timeout: 5), "Cancel must preserve the copy")
        guard reveal(copy, in: app) else { return }
        copy.press(forDuration: 1)
        app.buttons["Delete list"].tap()
        let confirmDelete = app.buttons["Delete List"]
        XCTAssertTrue(confirmDelete.waitForExistence(timeout: 5))
        let confirmReady = XCTNSPredicateExpectation(predicate: NSPredicate(format: "hittable == true"), object: confirmDelete)
        XCTAssertEqual(XCTWaiter.wait(for: [confirmReady], timeout: 5), .completed)
        confirmDelete.tap()
        let removed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: copy)
        XCTAssertEqual(XCTWaiter.wait(for: [removed], timeout: 5), .completed)
        XCTAssertTrue(renamed.exists, "Deleting the duplicate must preserve the original list")
        guard reveal(renamed, in: app) else { return }
        renamed.tap()
        XCTAssertTrue(app.buttons["Actions for Low Tide"].waitForExistence(timeout: 5), "Original membership must survive rename, duplication and copy deletion")
    }

    func testLongPressCancelsPrimaryTapAndShortTapStillOpensList() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "lists"
        app.launch()
        let list = app.staticTexts["Late drive"].firstMatch
        XCTAssertTrue(list.waitForExistence(timeout: 25))
        guard reveal(list, in: app, fullyInsideScroll: true) else { return }
        let dragStart = list.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
        dragStart.press(forDuration: 0.1, thenDragTo: dragStart.withOffset(CGVector(dx: 0, dy: -80)))
        XCTAssertFalse(app.buttons["Rename…"].exists, "Moving beyond the hold tolerance must cancel the context menu")
        XCTAssertFalse(app.buttons["Actions for Low Tide"].exists, "A scrolling gesture must not open the list")
        guard reveal(list, in: app, fullyInsideScroll: true) else { return }
        list.press(forDuration: 0.7)
        let rename = app.buttons["Rename…"]
        XCTAssertTrue(rename.waitForExistence(timeout: 5))
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.03, dy: 0.08)).tap()
        let dismissed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: rename)
        XCTAssertEqual(XCTWaiter.wait(for: [dismissed], timeout: 5), .completed)
        XCTAssertTrue(app.staticTexts["Automatic"].exists, "A successful long press must not perform the list's primary navigation")
        XCTAssertFalse(app.buttons["Back"].firstMatch.exists)
        guard reveal(list, in: app, fullyInsideScroll: true) else { return }
        list.tap()
        XCTAssertTrue(app.buttons["Actions for Low Tide"].waitForExistence(timeout: 5), "A short tap must retain its original navigation")
        app.buttons["Find"].firstMatch.tap()
        let search = app.textFields["Songs, albums, folders"]
        XCTAssertTrue(search.waitForExistence(timeout: 5))
        search.tap()
        search.typeText("cloud")
        XCTAssertTrue(app.keyboards.firstMatch.waitForExistence(timeout: 5))
        let songActions = app.buttons["Actions for Harbour"].firstMatch
        XCTAssertTrue(songActions.waitForExistence(timeout: 8))
        songActions.tap()
        XCTAssertTrue(app.buttons["Share…"].waitForExistence(timeout: 5))
        let keyboardGone = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: app.keyboards.firstMatch)
        XCTAssertEqual(XCTWaiter.wait(for: [keyboardGone], timeout: 5), .completed, "The context menu must make its lower actions reachable above the keyboard")
        XCTAssertTrue(app.buttons["Share…"].isHittable)
    }

    func testNewListFromSongSubmenuPreservesActualMembership() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "songs"
        app.launch()
        let actions = app.buttons["Actions for Antenna"].firstMatch
        XCTAssertTrue(actions.waitForExistence(timeout: 25))
        attach(app, "Song menu backdrop baseline")
        actions.tap()
        XCTAssertTrue(app.buttons["Add to list"].waitForExistence(timeout: 5))
        Thread.sleep(forTimeInterval: 0.7)
        attach(app, "Song context menu")
        app.buttons["Add to list"].tap()
        XCTAssertTrue(app.buttons["New list…"].waitForExistence(timeout: 5))
        Thread.sleep(forTimeInterval: 0.7)
        attach(app, "Add to list submenu")
        app.buttons["‹ Add to list"].tap()
        XCTAssertTrue(app.buttons["Add to list"].waitForExistence(timeout: 5))
        Thread.sleep(forTimeInterval: 0.7)
        attach(app, "Song menu after submenu Back")
        app.buttons["Add to list"].tap()
        XCTAssertTrue(app.buttons["New list…"].waitForExistence(timeout: 5))
        app.buttons["New list…"].tap()
        let name = app.textFields["List name"]
        XCTAssertTrue(name.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["1 song will be added."].exists)
        attach(app, "New List from song dialog")
        name.typeText("Single track list")
        app.buttons["Create"].tap()
        let closed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: name)
        XCTAssertEqual(XCTWaiter.wait(for: [closed], timeout: 5), .completed)
        app.buttons["Lists"].firstMatch.tap()
        let created = app.staticTexts["Single track list"].firstMatch
        guard reveal(created, in: app) else { return }
        XCTAssertTrue(created.waitForExistence(timeout: 10))
        created.tap()
        XCTAssertTrue(app.buttons["Actions for Antenna"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["1 songs"].exists, "The new list must contain exactly the selected saved song")
        app.buttons["Library"].firstMatch.tap()
        let actionsAgain = app.buttons["Actions for Antenna"].firstMatch
        XCTAssertTrue(actionsAgain.waitForExistence(timeout: 10))
        actionsAgain.tap()
        app.buttons["Add to list"].tap()
        app.buttons["Single track list"].tap()
        XCTAssertTrue(app.staticTexts["Already in “Single track list”"].waitForExistence(timeout: 2))
        XCTAssertFalse(app.alerts["LocalWave"].exists, "Adding an existing member must not report a repository error")
        app.buttons["Lists"].firstMatch.tap()
        guard reveal(created, in: app) else { return }
        created.tap()
        XCTAssertTrue(app.staticTexts["1 songs"].waitForExistence(timeout: 5), "A repeated add must preserve the single membership")
    }

    func testNewEmptyListOpensDetailAndUsesUntitledFallback() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "lists"
        app.launch()
        let create = app.buttons["New list"].firstMatch
        XCTAssertTrue(app.staticTexts["Automatic"].waitForExistence(timeout: 25))
        guard reveal(create, in: app, fullyInsideScroll: true) else { return }
        let scrolls = app.scrollViews.allElementsBoundByIndex.filter { $0.exists && $0.isHittable && $0.frame.width > 100 && $0.frame.height > 100 }
        guard let scroll = scrolls.first else { XCTFail("No active outer scroll viewport"); return }
        let viewport = scroll.frame.intersection(app.frame).insetBy(dx: 4, dy: 4)
        XCTAssertTrue(viewport.contains(create.frame), "The complete New list control must be inside the active viewport before tapping")
        XCTAssertTrue(create.isHittable)
        let geometry = XCTAttachment(string: "New list frame=\(create.frame) active viewport=\(viewport) scroll frames=\(scrolls.map(\.frame))")
        geometry.name = "New List pre-tap geometry"
        geometry.lifetime = .keepAlways
        add(geometry)
        attach(app, "New List control before tap")
        create.tap()
        let field = app.textFields["List name"]
        XCTAssertTrue(field.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Name your list."].exists)
        attach(app, "New List empty dialog")
        app.buttons["Create"].tap()
        XCTAssertTrue(app.staticTexts["Untitled list"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.buttons["List actions"].exists, "Empty-selection creation opens its new list detail")
        XCTAssertTrue(app.staticTexts["0 songs"].exists)
        XCTAssertTrue(app.staticTexts["Empty for now."].exists)
        XCTAssertFalse(app.buttons["Edit"].exists)
        attach(app, "New empty list detail")
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

}
