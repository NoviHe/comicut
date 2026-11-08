# ComiCut Studio (Rust + Tauri v2 + React)

Aplikasi otomatis untuk mengambil chapter komik (**Webtoon/Manhwa vertikal** & **Manga horizontal**), memotong panel secara cerdas tanpa membelah gambar karakter, memilih panel **Epic/Penting** (menggunakan *Rust Visual Heuristics* + *Vision AI*), menyinkronkannya dengan ketukan *sub-bass* lagu **Montagem / Phonk / Remix**, dan merendernya menjadi video vertikal **9:16 (1080x1920 60fps)** siap upload ke Reels, Shorts, atau TikTok.

---

## Fitur Utama

1. **Multi-Platform & Custom URL Scraper (`src-tauri/src/core/scraper/mod.rs`)**
   - **LINE Webtoon**: Mendukung ekstraksi `#_imageList img` (`data-url`) otomatis dengan injeksi header `Referer: https://www.webtoons.com/` anti-403.
   - **MangaDex (API v5)**: Mengambil gambar resolusi penuh langsung dari endpoint resmi `https://api.mangadex.org/at-home/server/{chapter_uuid}`.
   - **VoraToon (`https://voratoon.web.id/`)**: Preset bawaan yang mengekstrak gambar dari `#readerarea img`, `.reading-content img`, atribut `data-lazy-src` / `data-src`, maupun payload script JSON `ts_reader.run(...)`.
   - **Universal Custom Website Input**: Pengguna bebas memasukkan URL website komik/manhwa/manga lain beserta opsi *Custom CSS Selector* dan *Custom Referer*.

2. **Smart Comic Panel Slicer (`src-tauri/src/core/comic_slicer/`)**
   - **Webtoon Vertical Slicer**: Memindai baris piksel *gutter* (ruang kosong putih/hitam seragam di antara panel) dan menandai panel aksi vertikal panjang sebagai `VerticalPan` (*auto camera scroll*).
   - **Manga Page Slicer**: Mendukung urutan baca **RTL (Kanan-ke-Kiri)** dan **LTR (Kiri-ke-Kanan)**.

3. **Hybrid Epic Panel Reviewer (`src-tauri/src/core/epic_reviewer/`)**
   - Menghitung **Edge Density**, **Contrast Score**, dan **Aspect Ratio** di Rust untuk memberi label otomatis: `Hook`, `BuildUp`, `AuraCloseup`, `Climax`, atau `VerticalPan`.
   - Membuat **Contact Sheet Grid** untuk integrasi hemat token dengan Vision LLM (Gemini / OpenAI / Ollama).

4. **Montagem Audio Beat & Bass Drop Detector (`src-tauri/src/core/audio_beat/`)**
   - Menggunakan `symphonia` + `rustfft` untuk memfilter frekuensi **Sub-Bass (40Hz – 150Hz)** khas lagu Montagem/Phonk dan mendeteksi timestamp **Main Bass Drop** secara presisi.

5. **9:16 Auto-Director & FFmpeg Pipe Renderer (`src-tauri/src/core/video_renderer/`)**
   - Menyinkronkan pergantian panel tepat pada ketukan *bass* dengan efek **Zoom Punch**, **Camera Shake**, **Vertical Pan Scroll**, dan **Impact Flash**.

---

## Cara Menjalankan Proyek

### 1. Mode Development (Tauri v2 Desktop)
```bash
npm install
npm run tauri dev
```

### 2. Mode Preview UI Cepat (Vite)
```bash
npm run dev
```

### 3. Build Aplikasi Produksi
```bash
npm run tauri build
```
