#!/usr/bin/env python3
"""Build real, deterministic demo audio. Requires ffmpeg, ffprobe and macOS Swift.

No library database writes: catalog tags are embedded in the AAC files; the app
uses its ordinary source importer. Four encoders maximum; AVFoundation validates
all files, including the duration floor used by the production interface.
"""
import argparse
import concurrent.futures
import hashlib
import json
import math
import pathlib
import random
import shutil
import subprocess
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--workers', type=int, default=4)
    parser.add_argument('--validate-only', action='store_true')
    parser.add_argument('--output', type=pathlib.Path, default=ROOT / 'localwave/DemoLibrary')
    args = parser.parse_args()
    ffmpeg = shutil.which('ffmpeg')
    if not ffmpeg:
        raise SystemExit('ffmpeg is required')
    args.output.mkdir(parents=True, exist_ok=True)
    reference = json.loads((ROOT / 'localwave/ReferenceFixture/reference.json').read_text())
    albums = {a['id']: a for a in reference['albums']}
    started = time.monotonic()

    def encode(track):
        album = albums[track['al']]
        randomizer = random.Random(int.from_bytes(hashlib.sha256((track['id'] + 'snd').encode()).digest()[:8], 'big'))
        root = 110 * 2 ** (randomizer.randrange(7) / 12)
        expressions = []
        for index, multiple in enumerate([1, 1.5, 2, 2.52]):
            frequency = root * multiple * 2 ** ((randomizer.random() - .5) * 12 / 1200)
            phase = f'2*PI*{frequency:.12f}*t'
            wave = f'(2/PI)*asin(sin({phase}))' if index % 2 else f'sin({phase})'
            expressions.append(f'{.025 / (index + 1):.12f}*{wave}')
        resource = 'localwave-demo-' + track['id'] + '.m4a'
        target = args.output / resource
        duration = track['dur'] + .1  # Real tail; never override the stored duration.
        command = [ffmpeg, '-hide_banner', '-loglevel', 'error', '-y', '-f', 'lavfi', '-i',
            f'aevalsrc={"+".join(expressions)}:s=44100:d={duration:.3f}',
            '-i', str(ROOT / f'localwave/ReferenceFixture/reference-{album["id"]}.png'),
            '-map', '0:a:0', '-map', '1:v:0', '-af',
            f'lowpass=f=1400,tremolo=f=0.2:d=0.1,afade=t=in:d=0.4,afade=t=out:st={duration-.4:.3f}:d=0.4',
            '-c:a', 'aac', '-b:a', '32k', '-c:v', 'copy', '-disposition:v:0', 'attached_pic',
            '-fflags', '+bitexact', '-flags:a', '+bitexact', '-metadata', f'title={track["t"]}',
            '-metadata', f'artist={track["ar"]}', '-metadata', f'album={album["t"]}',
            '-metadata', f'album_artist={album["ar"]}', '-metadata', f'date={album["y"]}',
            '-metadata', f'track={track["n"]}', '-metadata', 'disc=1', str(target)]
        begin = time.monotonic()
        subprocess.run(command, check=True, capture_output=True)
        # A slash in an album title is a title, not an extra directory boundary.
        component = lambda value: value.replace('/', '∕').replace(':', '꞉')
        return dict(resource=resource, relativePath='/'.join([component(track['ar']), component(album['t']), f'{track["n"]:02d} {component(track["t"])}.m4a']),
            title=track['t'], artist=track['ar'], album=album['t'], albumArtist=album['ar'], year=album['y'], trackNumber=track['n'], discNumber=1,
            referenceSeconds=track['dur'], bytes=target.stat().st_size, sha256=hashlib.sha256(target.read_bytes()).hexdigest(), encodeSeconds=time.monotonic()-begin)

    catalog_path = args.output / 'localwave-demo-catalog.json'
    if args.validate_only:
        catalog = json.loads(catalog_path.read_text())
        tracks = catalog['tracks']
    else:
        with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, min(4, args.workers))) as executor:
            tracks = list(executor.map(encode, reference['tracks']))
        catalog = {'version': 1, 'tracks': [{k: v for k, v in t.items() if k != 'encodeSeconds'} for t in tracks]}
        catalog_path.write_text(json.dumps(catalog, indent=2, ensure_ascii=False) + '\n')
    for track in tracks:
        recording = args.output / track['resource']
        if recording.stat().st_size != track['bytes'] or hashlib.sha256(recording.read_bytes()).hexdigest() != track['sha256']:
            raise RuntimeError('Demo file validation failed: ' + track['resource'])
    # Native metadata/timing validation is mandatory, not inferred from ffprobe.
    swift = r'''
import AVFoundation
import Foundation
@main struct Validate {
 static func main() async throws {
  let directory = URL(fileURLWithPath: CommandLine.arguments[1])
  let catalog = try JSONSerialization.jsonObject(with: Data(contentsOf: directory.appendingPathComponent("localwave-demo-catalog.json"))) as! [String: Any]
  var results: [[String: Any]] = []
  for row in catalog["tracks"] as! [[String: Any]] {
   let asset = AVURLAsset(url: directory.appendingPathComponent(row["resource"] as! String))
   let seconds = try await asset.load(.duration).seconds
   guard seconds.isFinite, Int(floor(seconds)) == row["referenceSeconds"] as! Int else { throw NSError(domain: "Demo duration floor", code: 1, userInfo: row) }
   var tags: [String: String] = [:]; var artwork = false; var indices: [String: Int] = [:]
   for item in try await asset.load(.metadata) {
    if let value = try? await item.load(.stringValue), let id = item.identifier { tags[id.rawValue] = value }
    if item.commonKey == .commonKeyArtwork, let data = try? await item.load(.dataValue), !data.isEmpty { artwork = true }
    if let id = item.identifier, id == .iTunesMetadataTrackNumber || id == .iTunesMetadataDiscNumber,
       let data = try? await item.load(.dataValue) {
     let bytes = Array(data.prefix(4)); if bytes.count == 4 { indices[id.rawValue] = Int(bytes[2]) * 256 + Int(bytes[3]) }
    }
   }
   let expected: [(AVMetadataIdentifier, String)] = [(.iTunesMetadataSongName, "title"), (.iTunesMetadataArtist, "artist"), (.iTunesMetadataAlbum, "album"), (.iTunesMetadataAlbumArtist, "albumArtist")]
   for (identifier, key) in expected {
    guard tags[identifier.rawValue] == row[key] as? String else { throw NSError(domain: "Demo tag " + key, code: 2, userInfo: row) }
   }
   guard Int(tags[AVMetadataIdentifier.iTunesMetadataReleaseDate.rawValue]?.prefix(4) ?? "") == row["year"] as? Int,
         indices[AVMetadataIdentifier.iTunesMetadataTrackNumber.rawValue] == row["trackNumber"] as? Int,
         indices[AVMetadataIdentifier.iTunesMetadataDiscNumber.rawValue] == row["discNumber"] as? Int,
         artwork else { throw NSError(domain: "Demo year/track/disc/artwork", code: 3, userInfo: row) }
   let audio = try AVAudioFile(forReading: directory.appendingPathComponent(row["resource"] as! String))
   guard audio.length > 0 else { throw NSError(domain: "Demo empty audio", code: 3) }
   results.append(["resource": row["resource"]!, "assetSeconds": seconds, "frames": audio.length])
  }
  FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: results, options: [.prettyPrinted, .sortedKeys]))
 }
}
'''
    with tempfile.TemporaryDirectory(prefix='localwave-demo-validate-') as temp:
        source = pathlib.Path(temp) / 'Validate.swift'
        source.write_text(swift)
        binary = pathlib.Path(temp) / 'validate'
        subprocess.run(['swiftc', '-parse-as-library', str(source), '-o', str(binary)], check=True)
        validated = json.loads(subprocess.check_output([str(binary), str(args.output)]))
    report = dict(trackCount=len(tracks), albumCount=len(albums), originalSeconds=sum(t['referenceSeconds'] for t in tracks),
        totalBytes=sum(t['bytes'] for t in tracks), catalogBytes=catalog_path.stat().st_size, bundleBytes=sum(t['bytes'] for t in tracks) + catalog_path.stat().st_size, wallSeconds=time.monotonic()-started, workers=max(1, min(4, args.workers)),
        encodeSeconds=sum(t.get('encodeSeconds', 0) for t in tracks), ffmpegVersion=subprocess.check_output([ffmpeg, '-version'], text=True).splitlines()[0], nativeValidation=validated)
    # Keep build evidence outside the app's flattened resource namespace.
    report_path = pathlib.Path('/tmp/localwave-demo-validation-report.json') if args.validate_only else args.output.parent.parent / 'demo-generation-report.json'
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'nativeValidation'}, indent=2))

if __name__ == '__main__':
    main()
