# Roadmap

## Release blockers

1. Full desktop interaction verification, different-user Windows pipe ACL denial testing, and symlink/reparse-point race hardening.
2. Broader process-kill and disk-fault injection beyond the tested finalization/database recovery boundary.
3. Local builds/tests on any additionally claimed platforms, dependency audit/license review, browser end-to-end validation.

## Next milestones

1. Broader validation of implemented queue settings, scheduled starts, categories, global bandwidth, batch controls and history recipes; pagination for large IPC histories and scalable queue dispatch.
2. Multi-selection, drag/drop, full Swedish localization, accessible keyboard navigation, notifications and tray.
3. Native browser registration installer, reliable errors for context-menu handoff, explicit authenticated requests.
4. Adaptive segmentation, broader measured benchmarks and slow-disk optimization.
5. Signing and upgrade testing for the implemented per-user NSIS installer; verified Linux packages and macOS app bundle/signing.

Future protocol adapters and authorized unencrypted HLS/DASH require separately validated parsers, segment assembly, and muxing. DRM bypass is outside scope.
