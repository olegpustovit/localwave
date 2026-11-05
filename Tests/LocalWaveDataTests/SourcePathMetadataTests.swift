import Foundation
import XCTest
import SQLite
import LocalWaveDomain
@testable import LocalWaveData

final class SourcePathMetadataTests: XCTestCase {
    func testLegacyPathTableGainsOptionalSizeWithoutLosingRecords() async throws {
        let database = try Connection(.inMemory)
        let old = Table("source_paths")
        let id = SQLite.Expression<Int64>("id"), source = SQLite.Expression<Int64>("sourceId"), path = SQLite.Expression<Int64>("pathId")
        let parent = SQLite.Expression<Int64?>("parentPathId")
        let name = SQLite.Expression<String>("name"), relative = SQLite.Expression<String>("relativePath")
        let directory = SQLite.Expression<Bool>("isDirectory"), hash = SQLite.Expression<Data?>("fileHashSHA256")
        let run = SQLite.Expression<Int64>("runId"), created = SQLite.Expression<Date>("createdAt"), updated = SQLite.Expression<Date?>("updatedAt")
        try database.run(old.create { table in
            table.column(id, primaryKey: .autoincrement); table.column(source); table.column(path); table.column(parent)
            table.column(name); table.column(relative); table.column(directory); table.column(hash); table.column(run); table.column(created); table.column(updated)
        })
        try database.run(old.insert(source <- 1, path <- 42, parent <- 1, name <- "cover.jpg", relative <- "cover.jpg", directory <- false, hash <- nil, run <- 1, created <- Date(timeIntervalSince1970: 123), updated <- nil))
        let repository = try SQLiteSourcePathRepository(db: database)
        let row = try await repository.getByPathId(sourceId: 1, pathId: 42)
        XCTAssertEqual(row?.name, "cover.jpg")
        XCTAssertNil(row?.fileSize)
        XCTAssertEqual(row?.createdAt, Date(timeIntervalSince1970: 123))
        XCTAssertFalse(try XCTUnwrap(row).isAudio)
    }
}
