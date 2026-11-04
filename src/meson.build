cargo_script = find_program(meson.project_source_root() / 'build-aux' / 'cargo.sh')

cargo_build = custom_target('cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: ['rgbeastd'],
  console: true,
  install: true,
  install_dir: [libexecdir],
  command: [
    cargo_script,
    meson.project_build_root(),
    meson.project_source_root(),
    '@OUTDIR@',
    get_option('buildtype'),
    get_option('offline') ? 'offline' : 'online',
  ],
)

# The app: the rgbeast_gui package under /usr/share/rgbeast and a launcher in /usr/bin.
gui_dir = meson.project_source_root() / 'rgbeast_gui'
install_subdir(gui_dir, install_dir: pkgdatadir, exclude_directories: ['__pycache__', 'qml/RGBeast/__pycache__'])

launcher_conf = configuration_data()
launcher_conf.set('pkgdatadir', pkgdatadir)
configure_file(
  input: 'rgbeast.in',
  output: 'rgbeast',
  configuration: launcher_conf,
  install: true,
  install_dir: bindir,
  install_mode: 'rwxr-xr-x',
)

# Pure-Python model tests always run; the Qt ones skip themselves when PySide6 is missing.
test('App unit tests', python,
  args: ['-m', 'unittest', 'discover', '-s', 'tests', '-t', '.'],
  workdir: meson.project_source_root(),
  env: ['QT_QPA_PLATFORM=offscreen', 'PYTHONDONTWRITEBYTECODE=1'],
  timeout: 120,
)
