import XCTest
import SQLite
import LocalWaveDomain
@testable import LocalWaveData

final class LocalUserTests: XCTestCase {
    func testConcurrentFirstLaunchResolvesOneLocalUser() async throws {
        let database = try Connection(.inMemory)
        let repository = try SQLiteUserRepository(db: database)
        let service = DefaultUserCloudService(userService: DefaultUserService(userRepository: repository), iCloudProvider: SignedOutProvider())
        let ids = try await withThrowingTaskGroup(of: Int64?.self) { tasks in
            for _ in 0..<32 { tasks.addTask { try await service.resolveCurrentICloudUser()?.id } }
            var ids: [Int64?] = []
            for try await id in tasks { ids.append(id) }
            return ids
        }
        XCTAssertEqual(ids.count, 32)
        XCTAssertEqual(Set(ids.compactMap { $0 }).count, 1)
        XCTAssertFalse(ids.contains(nil))
        XCTAssertEqual(try database.scalar("SELECT COUNT(*) FROM users") as? Int64, 1)
    }
    func testSignedOutLibraryUsesStableLocalUser() async throws {
        let repository = try SQLiteUserRepository(db: Connection(.inMemory))
        let service = DefaultUserCloudService(userService: DefaultUserService(userRepository: repository), iCloudProvider: SignedOutProvider())
        let first = try await service.resolveCurrentICloudUser()
        let second = try await service.resolveCurrentICloudUser()
        XCTAssertNotNil(first?.id)
        XCTAssertEqual(first?.icloudId, -1)
        XCTAssertEqual(first?.id, second?.id)
    }
}

private struct SignedOutProvider: ICloudProvider {
    func getCurrentICloudUserID() async throws -> Int64? { nil }
    func isICloudAvailable() -> Bool { false }
}
