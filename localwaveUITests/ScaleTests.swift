import XCTest

final class ScaleTests: XCTestCase {
    func testFiveThousandSongLibraryScrollAndFind() throws {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_UI_LIBRARY_COUNT"] = "5000"
        var seconds: [String: Double] = [:]
        let launchStarted = ProcessInfo.processInfo.systemUptime
        app.launch()
        defer { app.terminate() }
        seconds["launch_return"] = ProcessInfo.processInfo.systemUptime - launchStarted

        let mobileSongs = app.buttons["library-mode-Songs"]
        let songs = mobileSongs.waitForExistence(timeout: 2) ? mobileSongs : app.buttons["Songs"].firstMatch
        XCTAssertTrue(songs.waitForExistence(timeout: 15))
        songs.tap()
        if mobileSongs.exists {
            let count = app.descendants(matching: .any).matching(identifier: "library-song-count").firstMatch
            let loaded = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == %@", "5000"), object: count)
            XCTAssertEqual(XCTWaiter.wait(for: [loaded], timeout: 90), .completed)
        } else {
            let count = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", "5000 songs")).firstMatch
            XCTAssertTrue(count.waitForExistence(timeout: 90))
        }
        seconds["launch_to_full_library"] = ProcessInfo.processInfo.systemUptime - launchStarted
        XCTAssertTrue(app.buttons["Actions for Playlist Test Track"].waitForExistence(timeout: 10))
        attach(app, name: "5000-song library")

        let library = app.scrollViews.element(boundBy: mobileSongs.exists ? 0 : 1)
        XCTAssertTrue(library.exists)
        let scrollStarted = ProcessInfo.processInfo.systemUptime
        for _ in 0..<3 { library.swipeUp(velocity: .fast) }
        seconds["three_scroll_gestures"] = ProcessInfo.processInfo.systemUptime - scrollStarted
        attach(app, name: "5000-song library after scrolling")

        let find = app.buttons["Find"].firstMatch
        if find.exists && find.isHittable { find.tap() }
        else {
            app.buttons["Search"].tap()
            let commands = app.textFields["Search commands…"]
            XCTAssertTrue(commands.waitForExistence(timeout: 5))
            commands.tap()
            commands.typeText("Find")
            app.buttons["Find"].firstMatch.tap()
        }
        let query = app.textFields["Songs, albums, folders"]
        XCTAssertTrue(query.waitForExistence(timeout: 10))
        query.tap()
        let typingStarted = ProcessInfo.processInfo.systemUptime
        query.typeText("Scale Track 04999")
        seconds["exact_query_typing"] = ProcessInfo.processInfo.systemUptime - typingStarted
        let exactStarted = ProcessInfo.processInfo.systemUptime
        XCTAssertTrue(app.buttons["Actions for Scale Track 04999"].waitForExistence(timeout: 30))
        XCTAssertTrue(app.staticTexts["Songs · 1"].waitForExistence(timeout: 10))
        seconds["exact_result_after_typing"] = ProcessInfo.processInfo.systemUptime - exactStarted
        attach(app, name: "Search finds final imported track")

        app.buttons["Clear search"].tap()
        query.tap()
        query.typeText("Scale Track")
        let broadStarted = ProcessInfo.processInfo.systemUptime
        XCTAssertTrue(app.staticTexts["Songs · 4999"].waitForExistence(timeout: 30))
        seconds["4999_results_after_typing"] = ProcessInfo.processInfo.systemUptime - broadStarted
        XCTAssertFalse(app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", "Albums ·")).firstMatch.exists)
        let showAll = app.buttons["find-show-all-songs"]
        XCTAssertTrue(showAll.waitForExistence(timeout: 10))
        attach(app, name: "Paged Find previews 8 of 4999 matching tracks")
        showAll.tap()
        let expanded = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: showAll)
        XCTAssertEqual(XCTWaiter.wait(for: [expanded], timeout: 10), .completed)
        app.scrollViews.firstMatch.swipeDown(velocity: .fast)

        app.buttons["Clear search"].tap()
        query.tap()
        query.typeText("NoSuchTrackScale")
        let missingStarted = ProcessInfo.processInfo.systemUptime
        XCTAssertTrue(app.staticTexts["Songs · 0"].waitForExistence(timeout: 30))
        seconds["zero_results_after_typing"] = ProcessInfo.processInfo.systemUptime - missingStarted
        let data = try JSONSerialization.data(withJSONObject: seconds, options: [.prettyPrinted, .sortedKeys])
        let measurement = XCTAttachment(data: data, uniformTypeIdentifier: "public.json")
        measurement.name = "5000-song UI wall-clock measurements (seconds)"
        measurement.lifetime = .keepAlways
        add(measurement)
        print("LOCALWAVE_SCALE_SECONDS=" + String(decoding: data, as: UTF8.self))
    }

    private func attach(_ app: XCUIApplication, name: String) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
