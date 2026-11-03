# Fedora/RPM spec for Emoxidize — build with:
#   rpmbuild -ba packaging/fedora/emoxidize.spec  (sourced from a vendored tarball)
# or with packit/obs as preferred.

%global app_id io.github.hakergeniusz.Emoxidize

Name:           emoxidize
Version:        1.0.0~beta.1
Release:        1%{?dist}
Summary:        Single-instance GTK4 emoji picker
License:        MIT
URL:            https://github.com/hakergeniusz/emoxidize
# GitHub names tag tarballs with dashes (v1.0.0-beta.1); keep in sync with Version:
Source0:        %{url}/archive/refs/tags/v1.0.0-beta.1/%{name}-1.0.0-beta.1.tar.gz
# --with prebuilt skips the cargo build and packages a binary provided in
# SOURCES as %{name}-bin-%{_arch} (used to produce the release aarch64 rpm
# from the cross-compiled binary without an emulated build).
%bcond_with prebuilt

ExclusiveArch:  x86_64 aarch64
%if %{with prebuilt}
Source1:        %{name}-bin-%{_arch}
%endif
BuildRequires:  rust-packaging
BuildRequires:  cargo
BuildRequires:  (crate(gtk4/default) >= 0.11 with rust-packaging)
BuildRequires:  pkgconfig(gtk4) >= 4.12
BuildRequires:  pkgconfig(libadwaita-1) >= 1.4
BuildRequires:  systemd-rpm-macros

%global _description %{expand:
Emoxidize is a fast, single-instance emoji picker built with GTK4 and
libadwaita. It presents a searchable grid of all canonical Unicode emojis
grouped into Recents, Smileys, People, Animals, Food, Travel, Activities,
Objects, Symbols and Flags, with live search over CLDR names and keywords,
skin-tone support (including showing all tones at once) and a configurable
grid. Selecting an emoji copies it to the clipboard; with the optional GNOME
Shell paste bridge, pasting into the focused window is supported natively.}

%description %_description

%package        bridge
Summary:        GNOME Shell paste bridge for Emoxidize
BuildArch:      noarch
Requires:       gnome-shell >= 45
%description    bridge %_description

%prep
%autosetup -n %{name}-%{version_no_tilde}

%build
%if %{without prebuilt}
cargo build --release
%endif

%install
%if %{with prebuilt}
install -Dpm 755 %{SOURCE1} %{buildroot}%{_bindir}/emoxidize
%else
install -Dpm 755 target/release/emoxidize %{buildroot}%{_bindir}/emoxidize
%endif
install -Dpm 644 data/%{app_id}.desktop %{buildroot}%{_datadir}/applications/%{app_id}.desktop
install -Dpm 644 data/%{app_id}.metainfo.xml %{buildroot}%{_metainfodir}/%{app_id}.metainfo.xml
install -Dpm 644 data/%{app_id}.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg
install -Dpm 644 data/%{app_id}-symbolic.svg %{buildroot}%{_datadir}/icons/hicolor/symbolic/apps/%{app_id}-symbolic.svg
for size in 16 24 32 48 64 128 256; do
  install -Dpm 644 "data/icons/hicolor/${size}x${size}/apps/%{app_id}.png" \
      %{buildroot}%{_datadir}/icons/hicolor/${size}x${size}/apps/%{app_id}.png
done
install -Dpm 644 data/%{app_id}.gschema.xml %{buildroot}%{_datadir}/glib-2.0/schemas/%{app_id}.gschema.xml
install -Dpm 644 extension/emoxidize@hakergeniusz/metadata.json \
    %{buildroot}%{_datadir}/gnome-shell/extensions/emoxidize@hakergeniusz/metadata.json
install -Dpm 644 extension/emoxidize@hakergeniusz/extension.js \
    %{buildroot}%{_datadir}/gnome-shell/extensions/emoxidize@hakergeniusz/extension.js

%files
%license LICENSE
%doc README.md SPEC.md
%{_bindir}/emoxidize
%{_datadir}/applications/%{app_id}.desktop
%{_datadir}/metainfo/%{app_id}.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg
%{_datadir}/icons/hicolor/symbolic/apps/%{app_id}-symbolic.svg
%{_datadir}/icons/hicolor/*/apps/%{app_id}.png
%{_datadir}/glib-2.0/schemas/%{app_id}.gschema.xml

%files bridge
%license LICENSE
%{_datadir}/gnome-shell/extensions/emoxidize@hakergeniusz/
