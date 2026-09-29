%global app_id io.github.djshiye.RGBeast
%global rgbeast_user rgbeast

Name:           rgbeast
Version:        1.0.1
Release:        1%{?dist}
Summary:        RGB lighting control for GNOME

License:        MIT
URL:            https://github.com/djshiye/RGBeast
Source0:        %{name}-%{version}.tar.gz
# cargo vendor --locked, from the same tree
Source1:        %{name}-%{version}-vendor.tar.xz

BuildRequires:  cargo-rpm-macros >= 24
BuildRequires:  meson >= 1.0
BuildRequires:  gcc
BuildRequires:  pkgconfig(gtk4) >= 4.14
BuildRequires:  pkgconfig(libadwaita-1) >= 1.5
BuildRequires:  pkgconfig(gio-2.0)
BuildRequires:  pkgconfig(libudev)
BuildRequires:  blueprint-compiler
BuildRequires:  desktop-file-utils
BuildRequires:  appstream
BuildRequires:  gettext
BuildRequires:  systemd-rpm-macros

Requires:       gtk4%{?_isa} >= 4.14
Requires:       libadwaita%{?_isa} >= 1.5
Requires:       hicolor-icon-theme
Requires:       polkit
Requires:       dbus-common

%description
RGBeast controls the RGB lighting of ASUS Aura motherboards and the addressable
fans on their headers, Kingston Fury DDR5 memory and ASUS graphics cards. A
small sandboxed system daemon (rgbeastd) is the only process that touches the
hardware; the GTK 4 app talks to it over D-Bus, guarded by polkit.

%prep
%autosetup -n %{name}-%{version} -a1
%cargo_prep -v vendor

%build
export CARGO_HOME="$PWD/.cargo"
export RUSTFLAGS="%{build_rustflags}"
%meson -Doffline=true -Drgbeast_user=%{rgbeast_user}
%meson_build

%install
%meson_install

%check
%meson_test

# The rgbeast user, udev rules, D-Bus policy and unit files are picked up by
# the sysusers, udev, dbus and systemd file triggers of Fedora's rpm.

%post
%systemd_post rgbeastd.service
# Load i2c-dev and apply the new udev rules now, so the first run works
# without a reboot, then start the service. (A personal package: Fedora's
# preset policy would normally decide whether a service is enabled.)
modprobe i2c-dev >/dev/null 2>&1 || :
udevadm control --reload >/dev/null 2>&1 || :
udevadm trigger --subsystem-match=hidraw --subsystem-match=i2c-dev >/dev/null 2>&1 || :
udevadm settle --timeout=5 >/dev/null 2>&1 || :
systemctl enable --now rgbeastd.service >/dev/null 2>&1 || :

%preun
%systemd_preun rgbeastd.service

%postun
%systemd_postun_with_restart rgbeastd.service

%files
%license LICENSE
%doc README.md docs/PLAN.md docs/PROTOCOLS.md docs/TESTING.md
%{_bindir}/%{name}
%{_libexecdir}/rgbeastd
%{_mandir}/man1/%{name}.1*
%{_mandir}/man8/rgbeastd.8*
%{_datadir}/applications/%{app_id}.desktop
%{_datadir}/glib-2.0/schemas/%{app_id}.gschema.xml
%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg
%{_datadir}/icons/hicolor/symbolic/apps/%{app_id}-symbolic.svg
%{_datadir}/metainfo/%{app_id}.metainfo.xml
%{_datadir}/dbus-1/system-services/io.github.djshiye.RGBeast1.service
%{_datadir}/dbus-1/system.d/io.github.djshiye.RGBeast1.conf
%{_datadir}/polkit-1/actions/io.github.djshiye.rgbeast.policy
%{_unitdir}/rgbeastd.service
%{_udevrulesdir}/70-rgbeast.rules
%{_sysusersdir}/rgbeast.conf
%{_modulesloaddir}/rgbeast.conf

%changelog
* Tue Sep 29 2026 djshiye <dreamfantom16@gmail.com> - 1.0.1-1
- Redesigned app chrome on libadwaita materials; daemon and discovery fixes

* Mon Sep 28 2026 djshiye <dreamfantom16@gmail.com> - 1.0.0-1
- First release
