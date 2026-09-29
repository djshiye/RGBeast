project('rgbeast', 'rust',
  version: '1.0.1',
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

dependency('gtk4', version: '>= 4.14')
dependency('libadwaita-1', version: '>= 1.5')
dependency('libudev')
find_program('blueprint-compiler')
find_program('glib-compile-resources')
find_program('glib-compile-schemas')
cargo = find_program('cargo')

subdir('data')
subdir('po')
subdir('src')

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)
