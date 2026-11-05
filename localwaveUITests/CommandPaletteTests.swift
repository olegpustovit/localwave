import XCTest

final class CommandPaletteTests: XCTestCase {
    func testCommandSearchAcceptsTypingAndRoutesToSettings() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "commands"
        app.launch()
        let search = app.textFields["Search songs, folders, or type a command"]
        XCTAssertTrue(search.waitForExistence(timeout: 25))
        XCTAssertFalse(app.buttons["library-mode-Albums"].exists, "The command modal must hide the underlying Library from accessibility")
        XCTAssertTrue(app.alerts["Search and commands"].exists, "Search must expose a modal accessibility role")
        XCTAssertFalse(app.buttons["Open now playing"].isHittable, "The modal must block interaction with the transport")
        XCTAssertTrue(app.keyboards.firstMatch.waitForExistence(timeout: 5))
        app.typeText("settings")
        XCTAssertEqual(search.value as? String, "settings")
        let settings = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "Settings")).firstMatch
        XCTAssertTrue(settings.waitForExistence(timeout: 5))
        settings.tap()
        XCTAssertTrue(app.staticTexts["Appearance"].waitForExistence(timeout: 5))
        XCTAssertFalse(search.exists)
        app.buttons["Library"].firstMatch.tap()
        XCTAssertTrue(app.buttons["library-mode-Albums"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts["Appearance"].exists)
    }

    func testSemanticSongSearchOpensAllResultsInFind() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "commands"
        app.launch()
        let search = app.textFields["Search songs, folders, or type a command"]
        XCTAssertTrue(search.waitForExistence(timeout: 25))
        search.tap()
        search.typeText("year:2019")
        XCTAssertEqual(search.value as? String, "year:2019")
        let all = app.buttons["Show all 8 songs"]
        XCTAssertTrue(all.waitForExistence(timeout: 8))
        all.tap()
        let find = app.textFields["Songs, albums, folders"]
        XCTAssertTrue(find.waitForExistence(timeout: 5))
        XCTAssertEqual(find.value as? String, "year:2019")
        XCTAssertTrue(app.buttons["Actions for Blue Hour"].waitForExistence(timeout: 5))
        XCTAssertFalse(search.exists)
    }

    func testShortcutCommandOpensKeyboardHelp() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "commands"
        app.launch()
        let search = app.textFields["Search songs, folders, or type a command"]
        XCTAssertTrue(search.waitForExistence(timeout: 25))
        search.tap()
        search.typeText("keyboard")
        app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "Show keyboard shortcuts")).firstMatch.tap()
        XCTAssertTrue(app.staticTexts["Keyboard"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Seek 5 s · 30 s"].exists)
        XCTAssertFalse(search.exists)
        XCTAssertFalse(app.buttons["library-mode-Albums"].exists, "Keyboard Help must hide the underlying Library from accessibility")
        XCTAssertTrue(app.descendants(matching: .any)["Keyboard shortcuts"].firstMatch.exists, "Keyboard Help must expose its named accessibility container")
        XCTAssertFalse(app.buttons["Open now playing"].isHittable, "Keyboard Help must block interaction with the transport")
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.02, dy: 0.08)).tap()
        XCTAssertTrue(app.buttons["library-mode-Albums"].waitForExistence(timeout: 5))
    }
}
