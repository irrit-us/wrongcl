import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wrongcl/theme/wrongcl_colors.dart';

void main() {
  test('forVariant returns the legacy palettes for the wrongcl variant', () {
    expect(
      WrongclColors.forVariant(WrongclThemeVariant.wrongcl, Brightness.light),
      same(WrongclColors.light),
    );
    expect(
      WrongclColors.forVariant(WrongclThemeVariant.wrongcl, Brightness.dark),
      same(WrongclColors.dark),
    );
  });

  test('forVariant returns Nord Snow Storm and Polar Night palettes', () {
    final light = WrongclColors.forVariant(
      WrongclThemeVariant.nord,
      Brightness.light,
    );
    final dark = WrongclColors.forVariant(
      WrongclThemeVariant.nord,
      Brightness.dark,
    );

    // nord6 Snow Storm scaffold
    expect(light.surface.scaffold, const Color(0xFFECEFF4));
    // nord10 Frost — the primary accent for Snow Storm
    expect(light.accent.primary, const Color(0xFF5E81AC));
    // nord0 Polar Night scaffold
    expect(dark.surface.scaffold, const Color(0xFF2E3440));
    // nord8 Frost — the primary accent for Polar Night
    expect(dark.accent.primary, const Color(0xFF88C0D0));
    // nord11 — Aurora red status across both
    expect(light.status.danger, const Color(0xFFBF616A));
    expect(dark.status.danger, const Color(0xFFBF616A));
  });

  test('forVariant returns Rosé Pine Dawn and Rosé Pine palettes', () {
    final light = WrongclColors.forVariant(
      WrongclThemeVariant.rosePineDawn,
      Brightness.light,
    );
    final dark = WrongclColors.forVariant(
      WrongclThemeVariant.rosePineDawn,
      Brightness.dark,
    );

    // Dawn base
    expect(light.surface.scaffold, const Color(0xFFFAF4ED));
    // Dawn text
    expect(light.text.primary, const Color(0xFF575279));
    // Dawn pine accent
    expect(light.accent.primary, const Color(0xFF286983));
    // Dawn love (danger)
    expect(light.text.danger, const Color(0xFFB4637A));
    // Rosé Pine main base
    expect(dark.surface.scaffold, const Color(0xFF191724));
    // Rosé Pine foam accent
    expect(dark.accent.primary, const Color(0xFF9CCFD8));
    // Rosé Pine love (danger)
    expect(dark.text.danger, const Color(0xFFEB6F92));
  });

  test('forVariant returns Catppuccin Latte and Mocha palettes', () {
    final light = WrongclColors.forVariant(
      WrongclThemeVariant.catppuccin,
      Brightness.light,
    );
    final dark = WrongclColors.forVariant(
      WrongclThemeVariant.catppuccin,
      Brightness.dark,
    );

    // Latte base
    expect(light.surface.scaffold, const Color(0xFFEFF1F5));
    // Latte text
    expect(light.text.primary, const Color(0xFF4C4F69));
    // Latte blue accent
    expect(light.accent.primary, const Color(0xFF1E66F5));
    // Latte red
    expect(light.status.danger, const Color(0xFFD20F39));
    // Mocha base
    expect(dark.surface.scaffold, const Color(0xFF1E1E2E));
    // Mocha text
    expect(dark.text.primary, const Color(0xFFCDD6F4));
    // Mocha blue
    expect(dark.accent.primary, const Color(0xFF89B4FA));
    // Mocha red
    expect(dark.status.danger, const Color(0xFFF38BA8));
  });

  test('variant id round-trips through fromId', () {
    for (final variant in WrongclThemeVariant.values) {
      expect(WrongclThemeVariantId.fromId(variant.id), variant);
    }
    expect(
      WrongclThemeVariantId.fromId('unknown-or-stale-id'),
      WrongclThemeVariant.wrongcl,
    );
    expect(
      WrongclThemeVariantId.fromId(null),
      WrongclThemeVariant.wrongcl,
    );
  });
}
