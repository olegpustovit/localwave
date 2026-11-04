import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  Tests.swift
//  musicappTests
//
//  Created by Oleg Pustovit on 10.01.2025.
//

import Testing

import Foundation
@testable import localwave

struct FileHelperTests {
    @Test func testToString() async throws {
        let url = URL(fileURLWithPath: "/Users/doggyman/Documents/myFile.txt")
        let helper = FileHelper(fileURL: url)
        #expect(helper.toString() == "file:///Users/doggyman/Documents/myFile.txt")
    }

    @Test func testName() async throws {
        let url = URL(fileURLWithPath: "/Users/doggyman/Documents/myFile.txt")
        let helper = FileHelper(fileURL: url)
        #expect(helper.name() == "myFile.txt")
    }

    @Test func testParent() async throws {
        let url = URL(fileURLWithPath: "/Users/doggyman/Documents/myFile.txt")
        let helper = FileHelper(fileURL: url)
        #expect(helper.parent()?.path == "/Users/doggyman/Documents")
    }

    @Test func testRelativePath() async throws {
        let baseURL = URL(fileURLWithPath: "/Users/doggyman/Documents")
        let fileURL = URL(fileURLWithPath: "/Users/doggyman/Documents/Folder/myFile.txt")
        let helper = FileHelper(fileURL: fileURL)
        #expect(helper.relativePath(from: baseURL) == "Folder/myFile.txt")
    }

    @Test func relativePathSamePath() async throws {
        let baseURL = URL(fileURLWithPath: "/Users/doggyman/Documents/Folder/myFile.txt")
        let fileURL = URL(fileURLWithPath: "/Users/doggyman/Documents/Folder/myFile.txt")
        let helper = FileHelper(fileURL: fileURL)
        #expect(helper.relativePath(from: baseURL) == "")
    }

    @Test func createURLWithEmptyRelativePath() async throws {
        let baseURL = URL(fileURLWithPath: "/Users/doggyman/Documents")
        let relativePath = ""
        let expectedURL = URL(fileURLWithPath: "/Users/doggyman/Documents")
        let createdURL = FileHelper.createURL(baseURL: baseURL, relativePath: relativePath)
        #expect(createdURL?.path == expectedURL.path)
    }

    @Test func testCreateURL() async throws {
        let baseURL = URL(fileURLWithPath: "/Users/doggyman/Documents")
        let relativePath = "Folder/myFile.txt"
        let expectedURL = URL(fileURLWithPath: "/Users/doggyman/Documents/Folder/myFile.txt")
        let createdURL = FileHelper.createURL(baseURL: baseURL, relativePath: relativePath)
        #expect(createdURL?.path == expectedURL.path)
    }
}
