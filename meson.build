project('rgbeast', 'rust',
  version: '1.1.0',
  meson_version: '>= 1.0.0',
  license: 'MIT',
  default_options: ['buildtype=release'],
)

gnome = import('gnome')
i18n = import('i18n')

app_id = 'io.github.djshiye.RGBeast'
prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
libexecdir = prefix / get_option('libexecdir')
datadir = prefix / get_option('datadir')

pkgdatadir = datadir / 'rgbeast'

# rgbeastd is Rust; the app is Python + Qt Quick (PySide6) and needs no build step.
dependency('libudev')
cargo = find_program('cargo')
python = find_program('python3')

subdir('data')
subdir('po')
subdir('src')

gnome.post_install(
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)
