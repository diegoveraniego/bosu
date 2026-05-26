# Bosu 🟢

<p align="center">
  <em>A Kinesiological Rehabilitation Tracker built natively for the GNOME Desktop.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/GTK4-7DF1FE?style=for-the-badge&logo=gtk&logoColor=black" />
  <img src="https://img.shields.io/badge/GNOME-4A86CF?style=for-the-badge&logo=gnome&logoColor=white" />
</p>

## Overview

**Bosu** is a specialized tracking application designed specifically for physical rehabilitation (such as dorsalgia and scapular dyskinesia). Unlike traditional fitness or HIIT apps that focus on speed and intensity, Bosu is built for **mindful, slow, and analytical tracking** of your daily physical therapy routines.

It is built completely in **Rust** using **GTK4** and **Libadwaita**, ensuring a gorgeous, native, and perfectly integrated experience on any modern Linux desktop (GNOME 45+).

## ✨ Features

- **Native GNOME Experience**: Fully compliant with the GNOME Human Interface Guidelines (HIG). Uses `AdwViewSwitcher`, `AdwPreferencesGroup`, and native symbolic iconography.
- **Intelligent Kinesiology Alerts**: Built-in analytics act as a virtual physical therapist. It analyzes your pain trends and uses `AdwBanner` to suggest increasing weights or consulting your doctor.
- **Activity Heatmap**: A GitHub-style monthly adherence heatmap that dynamically colors cells based on your completion percentage.
- **Progress Dashboard**: View your current streak, average pain levels, and a custom native Bar Chart tracking your pain vs. improvement over the last 7 sessions.
- **Local-First & Private**: Your health data is yours. Bosu saves everything locally in `~/.local/share/bosu_history.jsonl` using efficient JSON serialization. No cloud, no telemetry.

## 🛠️ Installation & Building

Make sure you have Rust and the GTK4 development libraries installed on your system.

### Prerequisites (Arch Linux)
```bash
sudo pacman -S rustup base-devel gtk4 libadwaita
```

### Build & Run
Clone the repository and run it via Cargo:
```bash
git clone https://github.com/diegoveraniego/bosu.git
cd bosu
cargo run --release
```

## 🧠 Motivation

This project was born out of a personal need to track specific kinesiologic exercises (like Scapular Dyskinesia rehabilitation) methodically. By tracking pain levels and completion rates visually, it's easier to maintain discipline and provide accurate feedback to physical therapists. 

## 📜 License

This project is licensed under the MIT License.
