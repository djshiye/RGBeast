cargo_script = find_program(meson.project_source_root() / 'build-aux' / 'cargo.sh')
install_data(meson.project_source_root() / 'build-aux' / 'rgbeast-upgrade', install_dir: bindir, install_mode: 'rwxr-xr-x')

cargo_build = custom_target('cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: ['rgbeast', 'rgbeastd'],
  console: true,
  install: true,
  install_dir: [bindir, libexecdir],
  command: [
    cargo_script,
    meson.project_build_root(),
    meson.project_source_root(),
    '@OUTDIR@',
    get_option('buildtype'),
    get_option('offline') ? 'offline' : 'online',
    prefix / get_option('localedir'),
  ],
)
