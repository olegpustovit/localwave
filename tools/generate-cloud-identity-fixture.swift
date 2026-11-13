// Run from the repo root: swift tools/generate-cloud-identity-fixture.swift > crates/lw_apple_sys/fixtures/cloud-identity.txt
import Foundation

let identity = "test-fixture-token-名前🦊" as NSString
let data = try NSKeyedArchiver.archivedData(withRootObject: identity, requiringSecureCoding: true)
let decoded = try NSKeyedUnarchiver.unarchivedObject(ofClass: NSString.self, from: data)
assert(decoded == identity)
let encoded = data.base64EncodedString()
var hash: UInt64 = 0xCBF2_9CE4_8422_2325
for byte in encoded.utf8 { hash = (hash ^ UInt64(byte)) &* 0x100_0000_01B3 }
print(encoded)
print(Int64(bitPattern: hash & 0x7FFF_FFFF_FFFF_FFFF))
