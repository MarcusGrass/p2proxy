import 'package:flutter/material.dart';

ThemeData colorScheme() {
  return ThemeData(
    colorScheme: ColorScheme(
      brightness: Brightness.light,
      primary: const Color(0xffcdd6f4),
      onPrimary: const Color(0xff1e1e2e),
      secondary: const Color(0xff6c7086),
      onSecondary: const Color(0xffbac2de),
      error: catRed,
      onError: const Color(0xffffffff),
      surface: const Color(0xff313244),
      surfaceContainerLow: const Color(0xff45475a),
      onSurface: const Color(0xffbac2de),
    ),
  );
}

const Color catRed = Color(0xfff38ba8);
const Color catOnRed = Color(0xff45475a);
const Color catGreen = Color(0xffa6e3a1);

const Color _defaultTextColor = Color(0xffcdd6f4);
const TextStyle defaultText = TextStyle(color: _defaultTextColor);

const Color appBarColor = Color.fromARGB(255, 108, 112, 134);
