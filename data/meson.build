desktop_file = i18n.merge_file(
  input: app_id + '.desktop.in',
  output: app_id + '.desktop',
  type: 'desktop',
  po_dir: '../po',
  install: true,
  install_dir: datadir / 'applications',
)
desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils, args: [desktop_file])
endif

appstream_file = i18n.merge_file(
  input: app_id + '.metainfo.xml.in',
  output: app_id + '.metainfo.xml',
  po_dir: '../po',
  install: true,
  install_dir: datadir / 'metainfo',
)
appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test('Validate appstream file', appstreamcli, args: ['validate', '--no-net', '--explain', appstream_file])
endif

install_data(app_id + '.gschema.xml', install_dir: datadir / 'glib-2.0' / 'schemas')
compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test('Validate schema file', compile_schemas, args: ['--strict', '--dry-run', meson.current_source_dir()])
endif

install_data('icons/hicolor/scalable/apps' / app_id + '.svg',
  install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps')
install_data('icons/hicolor/symbolic/apps' / app_id + '-symbolic.svg',
  install_dir: datadir / 'icons' / 'hicolor' / 'symbolic' / 'apps')

install_man('rgbeast.1')
install_man('rgbeastd.8')

# ── System integration for the daemon ─────────────────────────────────────
daemon_conf = configuration_data()
daemon_conf.set('libexecdir', libexecdir)
daemon_conf.set('rgbeast_user', get_option('rgbeast_user'))

configure_file(
  input: 'rgbeastd.service.in',
  output: 'rgbeastd.service',
  configuration: daemon_conf,
  install: true,
  install_dir: prefix / 'lib' / 'systemd' / 'system',
)
configure_file(
  input: 'io.github.djshiye.RGBeast1.service.in',
  output: 'io.github.djshiye.RGBeast1.service',
  configuration: daemon_conf,
  install: true,
  install_dir: datadir / 'dbus-1' / 'system-services',
)
configure_file(
  input: 'io.github.djshiye.RGBeast1.conf.in',
  output: 'io.github.djshiye.RGBeast1.conf',
  configuration: daemon_conf,
  install: true,
  install_dir: datadir / 'dbus-1' / 'system.d',
)
configure_file(
  input: '70-rgbeast.rules.in',
  output: '70-rgbeast.rules',
  configuration: daemon_conf,
  install: true,
  install_dir: prefix / 'lib' / 'udev' / 'rules.d',
)
configure_file(
  input: 'rgbeast.sysusers.conf.in',
  output: 'rgbeast.conf',
  configuration: daemon_conf,
  install: true,
  install_dir: prefix / 'lib' / 'sysusers.d',
)
install_data('rgbeast.modules-load.conf', rename: 'rgbeast.conf', install_dir: prefix / 'lib' / 'modules-load.d')

# polkit's ITS rules are not available everywhere, so the policy is installed untranslated.
configure_file(
  input: 'io.github.djshiye.rgbeast.policy.in',
  output: 'io.github.djshiye.rgbeast.policy',
  configuration: daemon_conf,
  install: true,
  install_dir: datadir / 'polkit-1' / 'actions',
)
