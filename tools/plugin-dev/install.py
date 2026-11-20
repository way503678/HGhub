"""Install a local HGhub skill archive without overwriting existing skills."""
import argparse
import pathlib
import shutil
import tempfile
import zipfile

def install(archive, destination):
    destination = pathlib.Path(destination).expanduser().resolve()
    target = destination / 'hghub-plugin-dev'
    if target.exists():
        raise SystemExit('Destination already exists: ' + str(target))
    with zipfile.ZipFile(archive) as package, tempfile.TemporaryDirectory() as staging:
        for info in package.infolist():
            name = pathlib.PurePosixPath(info.filename)
            if name.is_absolute() or '..' in name.parts or chr(92) in info.filename or not name.parts or name.parts[0] != 'hghub-plugin-dev':
                raise SystemExit('Archive contains an invalid path')
            if ((info.external_attr >> 16) & 0o170000) == 0o120000:
                raise SystemExit('Archive symlinks are unsupported')
        if sum(i.file_size for i in package.infolist()) > 16 * 1024 * 1024:
            raise SystemExit('Archive exceeds expected size')
        package.extractall(staging)
        source = pathlib.Path(staging) / 'hghub-plugin-dev'
        if not (source / 'SKILL.md').is_file():
            raise SystemExit('Missing SKILL.md')
        destination.mkdir(parents=True, exist_ok=True)
        shutil.copytree(source, target)
    print(target)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', required=True)
    parser.add_argument('--destination', required=True)
    args = parser.parse_args()
    install(args.archive, args.destination)
