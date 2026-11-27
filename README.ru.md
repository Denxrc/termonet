<div align="center">

```text
▀█▀ █▀▀▀ █▀▀█ █▀▄▀█ █▀▀█ █▄ █ █▀▀▀ ▀█▀
 █  █▀▀  █▄▄▀ █ █ █ █  █ █ ▀█ █▀▀   █ 
 █  █▄▄▄ █  █ █   █ █▄▄█ █  █ █▄▄▄  █ 
```

**Быстрый и динамический генератор системных тем и палитр Material You для Linux**

[![CI](https://img.shields.io/github/actions/workflow/status/Denxrc/termonet/ci.yml?branch=master&label=CI&logo=githubactions&logoColor=white)](https://github.com/Denxrc/termonet/actions/workflows/ci.yml)
[![Release](https://img.shields.io/badge/release-v1.6.0-blue?logo=github&logoColor=white)](https://github.com/Denxrc/termonet/releases)
[![Rust](https://img.shields.io/badge/rust-edition_2024-orange?logo=rust&logoColor=white)](Cargo.toml)
[![Platform](https://img.shields.io/badge/linux-wayland_%7C_x11-blueviolet?logo=linux&logoColor=white)]()
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![XERJ Corpus Hub автор](https://hub.xerj.org/contributors/Denxrc/badge.svg)](https://hub.xerj.org/contributors/Denxrc)

[English](README.md) · [Русский](README.ru.md) · [Архитектура](ARCHITECTURE.ru.md) · [Релизы](https://github.com/Denxrc/termonet/releases) · [Баг-репорт](.github/ISSUE_TEMPLATE/bug_report.md)

</div>

---

## ▸ Обзор

**Termonet** извлекает доминантные цвета из активных обоев рабочего стола (или заданного HEX-цвета), рассчитывает гармонизированные светлую и тёмную схемы **Material You (Material 3)** вместе с 16-цветной ANSI-палитрой терминала и аккуратно внедряет их в конфиги приложений без повреждения пользовательских настроек. Утилита работает без root-прав и интерпретаторов вроде Python или Node.js, объединяя интерактивный TUI на `ratatui` и автономный CLI со скоростью отклика < 5 мс.

> **Статус (`v1.6.0`)**: Интерактивный 4-шаговый TUI и мастер настройки, внешний реестр шаблонов ([`Denxrc/termonet-apps`](https://github.com/Denxrc/termonet-apps) + свои `~/.config/termonet/repos/*.toml`), мультимониторное определение обоев (`awww`, `swww`, `hyprpaper`, `wpaperd`, `swaybg`, GNOME, KDE Plasma, `feh`, `nitrogen`), шаблонизатор MiniJinja и headless CLI (`--apply`, `--toggle-theme`, `--theme`, `-c/--color`, `--install`) готовы к работе. Готовые статические сборки для Linux (`x86_64` и `aarch64`) публикуются в [GitHub Releases](https://github.com/Denxrc/termonet/releases).

---

## ▸ Быстрый старт

Скачайте готовый статический бинарник (`x86_64` или `aarch64`, без привязки к версии `glibc`), проверьте контрольную сумму и запустите Termonet:

```bash
# 1. Скачивание и проверка v1.6.0 (поддерживает x86_64 и aarch64 через uname -m)
VERSION="v1.6.0" ARCH="$(uname -m)"
curl -LO "https://github.com/Denxrc/termonet/releases/download/${VERSION}/termonet-${VERSION}-linux-${ARCH}.tar.gz"
curl -LO "https://github.com/Denxrc/termonet/releases/download/${VERSION}/SHA256SUMS.txt"
sha256sum --ignore-missing -c SHA256SUMS.txt
mkdir -p termonet-pkg && tar -xzf "termonet-${VERSION}-linux-${ARCH}.tar.gz" -C termonet-pkg
install -Dm755 termonet-pkg/termonet ~/.local/bin/termonet

# 2. Запуск интерактивного TUI (при первом запуске откроется мастер настройки)
termonet

# 3. Автономное применение и мгновенное переключение тёмной/светлой темы
termonet --apply
termonet --toggle-theme
```

---

## ▸ Принципы и возможности

| Принцип | Реализация в `v1.6.0` |
| :--- | :--- |
| **Ядро Material You** | `material-colors` (`Variant::Content`) сохраняет естественные оттенки в 19 семантических ролях MD3 и светлой/тёмной ANSI 16 палитрах через HCT. |
| **Быстрая обработка** | Сжатие в памяти до 128×128 (`image`), извлечение первого кадра GIF/WebP, квантование Celebi и zero-alloc сериализация HEX/RGB. |
| **Внешний реестр шаблонов** | Чистый бинарник без вшитых конфигов; официальный [`termonet-apps`](https://github.com/Denxrc/termonet-apps) + свои `repos/*.toml` поверх `ureq`/`rustls`, проверка SHA-256 и еженедельное автообновление. |
| **Адаптивный TUI и мастер** | 4-шаговый интерфейс на `ratatui` с 5px логотипом из полублоков Unicode, локализацией `RU`/`EN`, общим каталогом программ, модалками предупреждений и переключением мониторов (`Tab`/`m`). |
| **Поиск обоев везде** | Параллельный опрос (`std::thread::scope`) для Wayland (`awww`, `swww`, `hyprpaper` сокет/конфиг, `wpaperd`, `swaybg` `/proc`), GNOME, KDE Plasma и X11 (`feh`, `nitrogen`). |
| **Безопасные шаблоны** | Встроенный MiniJinja с изолированными маркерами (`>>> termonet colors >>>`), заменой старых импортов Pywal и вечным бэкапом `.original` при включении. |
| **Симлинки и кэш состояния** | `safe_write()` сохраняет симлинки и права доступа, пропускает лишние перезаписи, а `state.toml` исключает холостые срабатывания (`< 5 мс` на `--toggle-theme`). |

---

## ▸ Архитектура

```text
Демон обоев / -w ПУТЬ / -c #HEX
        │
        ▼
┌─────────────────────────────────────────────┐
│  Ядро Termonet (src/core · src/wallpaper)   │  Чистый Rust · Без root · ~3.5 МБ
│  · Параллельные провайдеры и санитайзер     │
│  · 128x128 -> Celebi -> CAM16 + HCT         │
│  · Защита от холостых запусков (state.toml) │
└─────────────────────────────────────────────┘
        │
        ├──► Интерактивный TUI (src/tui: Палитра -> Программы -> Хуки -> Применение)
        └──► Автономный CLI (src/cli: --apply / --toggle-theme / --theme / -c)
                │
                ▼
     MiniJinja -> safe_write() + маркеры -> Отвязанные хуки перезапуска
```

> [!TIP]
> Подробный разбор всех подсистем, Mermaid-диаграммы пайплайнов, таймауты IPC, 6 инвариантов безопасности и руководства по расширению: **[ARCHITECTURE.ru.md](ARCHITECTURE.ru.md)** ([English](ARCHITECTURE.md)).

<details>
<summary>Структура репозитория и модулей (8 модульных подсистем)</summary>

```text
termonet/
├── Cargo.toml                  # Манифест (Edition 2024, Thin LTO, strip = true, panic = "abort")
└── src/
    ├── main.rs                 # Точка входа, строгий парсер CLI-аргументов и диспетчер
    ├── lib.rs & error.rs       # Корень крейта и типизированная ошибка TermonetError (thiserror)
    ├── cli/                    # Автономный исполнитель (--apply, --toggle-theme, --theme, -c/--color)
    ├── config/                 # Пути XDG, config.toml (Schema v3), детектор темы, state.toml
    ├── core/                   # Обработка картинок, HEX-парсер RgbColor, 19 ролей MD3 + ANSI 16
    ├── registry/               # Клиент реестра (ureq/rustls/sha2), каталог, защита установки и откат
    ├── wallpaper/              # Трейт WallpaperProvider, бэкенды Wayland/DE/X11, выбор монитора
    ├── template/               # Движок MiniJinja, атомарный safe_write(), бэкапы, инъекция маркеров
    ├── tui/                    # Цикл событий Ratatui, TerminalGuard RAII, 5px PixelFont, 6 экранов
    └── i18n/                   # Словари локализации (русский и английский языки)
```

</details>

---

## ▸ CLI, TUI и интеграция с композиторами

### Основные команды

```bash
termonet                                # Запуск интерактивного TUI (автоопределение обоев)
termonet --apply                        # Автономно применить палитру из активных обоев и выйти
termonet --toggle-theme                 # Мгновенно переключить тёмную/светлую тему (< 5 мс из кэша)
termonet --theme dark                   # Явно установить и применить тему ('dark' или 'light')
termonet -c "#385e72" -a                # Сгенерировать и применить палитру от HEX-цвета
termonet --apply -w ~/Pictures/wall.png # Применить конкретный файл обоев в автономном режиме
termonet --list-rules                   # Список установленных, локальных и доступных программ
termonet --install kitty                # Показать предупреждения, проверить SHA-256 и установить Kitty
termonet --update-rules                 # Синхронизировать репо и обновить неизменённые правила
termonet --debug-wallpaper              # Вывести диагностический отчёт всех менеджеров обоев
```

<details>
<summary>Полная справка параметров командной строки (--help)</summary>

```text
Использование: termonet [ОПЦИИ] [ПУТЬ_К_ОБОЯМ]

Параметры автономного режима / автоматизации:
  -a, --apply              Автономный режим: применить тему из активных обоев и выйти
  -f, --force              Принудительное применение или перезапись изменённых правил
  -c, --color <HEX>        Сгенерировать палитру напрямую из HEX-цвета (например, '#385e72')
      --toggle-theme       Мгновенно переключить тему (тёмная/светлая) и применить
      --theme <РЕЖИМ>      Установить и применить тему ('dark' или 'light')

Управление реестром и шаблонами:
      --list-rules         Показать установленные, локальные и доступные в репо программы
      --install <ID>       Установить правило и шаблон из репозитория (поддерживает repo/id)
      --remove-rule <ID>   Удалить отслеживаемые файлы правила и шаблона
      --update-rules       Синхронизировать репо и обновить неизменённые правила
      --reset-rules        Принудительно сбросить отслеживаемые правила с .bak
      --sync-repos         Обновить локальный кэш индексов без установки программ
  -y, --yes                Подтверждать установку/удаление без интерактивного запроса

Общие параметры:
  -w, --wallpaper <ПУТЬ>   Путь к изображению обоев (PNG, JPG, GIF, WebP)
  -i, --wizard, --init     Запустить мастер начальной настройки
      --no-backup          Отключить создание резервных копий (.bak, .original)
      --backup             Включить создание резервных копий (.bak, .original)
      --debug-wallpaper    Вывести диагностический отчёт менеджеров обоев
  -V, --version            Показать версию программы
  -h, --help               Показать справочную информацию
```

</details>

<details>
<summary>Экраны TUI и горячие клавиши</summary>

| Шаг | Экран | Назначение |
| :---: | :--- | :--- |
| **1** | **Палитра** | Просмотр seed-цвета, 19 ролей MD3 и 16 цветов ANSI; переключение экранов или ввод пути/HEX |
| **2** | **Программы** | Выбор локальных программ или установка `[↓]` из реестра с превью предупреждений |
| **3** | **Хуки** | Включение и отключение команд перезапуска (`hyprctl reload`, `killall -SIGUSR2 waybar`) |
| **4** | **Применение** | Сводка планируемых записей и отвязанных хуков перед сохранением на диск |

| Клавиша | Действие |
| :--- | :--- |
| `←` / `→` (`h` / `l`) | Переход между шагами (1..4) |
| `1`, `2`, `3`, `4` | Прямой переход к шагу 1..4 |
| `↑` / `↓` (`k` / `j`) | Навигация по спискам и пунктам мастера |
| `Пробел` | Включить / отключить программу или установить `[↓]` из каталога |
| `u` | Синхронизировать репозитории и обновить установленные правила (шаг 2) |
| `d` / `Delete` | Удалить скачанные файлы отслеживаемой программы (шаг 2) |
| `Enter` | Применить тему (шаг 4) / Подтвердить или сохранить (мастер / ввод) |
| `Tab` / `m` | Переключение между мониторами (шаг 1, при разных обоях) |
| `w` / `o` | Открыть ввод пути к обоям или HEX-цвета (`#385e72`, `0x...`) |
| `r` | Повторно автоопределить обои из активного демона (шаг 1) |
| `s` / `F2` | Открыть мастер настроек |
| `Esc` | Закрыть окно ввода / откатить настройки мастера / выйти с главного экрана |
| `q` / `Ctrl+C` | Выход из Termonet |

</details>

<details>
<summary>Рецепты для композиторов, хуков обоев и systemd (Hyprland, Sway, waypaper, systemd)</summary>

Поскольку Termonet не висит в памяти фоновым демоном и использует `~/.local/state/termonet/state.toml` для пропуска холостых запусков, его удобно привязать к горячим клавишам, хукам или таймеру:

#### Hyprland (`~/.config/hypr/hyprland.conf` / `hyprland.lua`)

```ini
exec-once = termonet --apply
bind = $mainMod, T, exec, termonet --toggle-theme
```

#### Sway (`~/.config/sway/config`)

```ini
exec termonet --apply
bindsym $mod+t exec termonet --toggle-theme
```

#### Скрипт смены обоев / хук `waypaper`

```bash
# В скрипте смены обоев для awww / swww
awww img "$1" && termonet --apply -w "$1"
```

```ini
# ~/.config/waypaper/config.ini
post_command = termonet --apply -w $wallpaper
```

#### Пользовательский сервис и таймер `systemd`

```ini
# ~/.config/systemd/user/termonet.service
[Unit]
Description=Update Material You theme from active desktop wallpaper

[Service]
Type=oneshot
ExecStart=%h/.local/bin/termonet --apply
```

```ini
# ~/.config/systemd/user/termonet.timer
[Unit]
Description=Periodically check active wallpaper for Termonet

[Timer]
OnBootSec=10s
OnUnitActiveSec=30s

[Install]
WantedBy=timers.target
```

```bash
systemctl --user enable --now termonet.timer
```

</details>

---

## ▸ Конфигурация, репозитории и свои шаблоны

Конфигурация с подробными комментариями хранится в `~/.config/termonet/config.toml` (схема v3; пропущенные ключи подставляются по умолчанию, а минимальные конфиги без автозаголовка не перезаписываются), источники шаблонов — в `~/.config/termonet/repos/*.toml`, а манифест установленных программ и кэш индексов — в `~/.local/state/termonet/installed.toml` и `~/.local/state/termonet/registry_cache/`:

```toml
language = "ru"              # "ru" | "en"
tui_theme = "dark"           # "dark" | "light" | "system"
font_mode = "pixel"          # "pixel" | "standard"
display_server = "wayland"   # "wayland" | "x11"
wallpaper_backend = "auto"   # "auto", "awww", "swww", "hyprpaper", "wpaperd", "swaybg", "gnome", "kde", "feh", "nitrogen", "manual"
enabled_programs = []
auto_update_rules = true     # Еженедельная проверка (раз в 7 дней) при старте TUI или --apply
create_backups = false
```

<details>
<summary>Полная комментированная схема config.toml (Schema v3) и свои репозитории</summary>

```toml
# ==============================================================================
# Termonet Configuration
# Путь: ~/.config/termonet/config.toml
# Schema Version: 3
# ==============================================================================

language = "ru"
tui_theme = "dark"
font_mode = "pixel"
display_server = "wayland"
wallpaper_backend = "auto"

# Главный монитор (раскомментируйте для указания)
# primary_monitor = "eDP-1"

# Ручной путь к обоям (используется при wallpaper_backend = "manual" или как fallback)
# manual_wallpaper_path = "/path/to/wallpaper.png"

enabled_programs = []
auto_update_rules = true
create_backups = false
```

Чтобы подключить свой репозиторий шаблонов, добавьте `.toml`-файл в `~/.config/termonet/repos/` (например, `my-rice.toml`):

```toml
name = "my-rice"
url = "https://raw.githubusercontent.com/<user>/<repo>/main"
priority = 100
enabled = true
```

</details>

<details>
<summary>Создание своих правил, шаблонов, переменные и безопасные бэкапы</summary>

Поддержка любой программы задаётся двумя файлами — правилом (`~/.config/termonet/rules/<app>.toml`) и шаблоном MiniJinja (`~/.config/termonet/templates/<app>.tmpl`):

1. **Файл правила (`~/.config/termonet/rules/foot.toml`)**:
   ```toml
   version = 2
   name = "Foot"
   template = "foot.tmpl"
   output_path = "~/.config/foot/colors-termonet.ini"
   target_config = "~/.config/foot/foot.ini"
   import_line = "include = ~/.config/foot/colors-termonet.ini"
   import_position = "bottom"   # "top" | "bottom"
   comment_style = "hash"       # опционально: "hash" | "blockcss" | "slash" | "doubledash"
   reload_cmd = ""
   ```

2. **Файл шаблона (`~/.config/termonet/templates/foot.tmpl`)**:
   ```ini
   # termonet-version: 4
   [colors]
   background = {{ terminal.background.strip }}
   foreground = {{ terminal.foreground.strip }}
   regular0 = {{ terminal.color0.strip }}
   regular1 = {{ terminal.color1.strip }}
   ...
   bright7 = {{ terminal.color15.strip }}
   ```

3. **Доступные переменные и фильтры**:
   - **Активная тема (`active.*` / `theme.*`)** и явные **`dark.*`** / **`light.*`**: `primary`, `on_primary`, `primary_container`, `on_primary_container`, `secondary`, `on_secondary`, `secondary_container`, `on_secondary_container`, `tertiary`, `on_tertiary`, `error`, `on_error`, `surface`, `on_surface`, `surface_variant`, `on_surface_variant`, `background`, `on_background`, `outline`.
   - **Терминальные ANSI (`terminal.*`, `terminal_dark.*`, `terminal_light.*`)**: `color0`..`color15`, `background`, `foreground`, `cursor`, а также **`seed_color`**.
   - **Свойства цвета и фильтры**: `.hex` (`#adc6ff`), `.strip` (`adc6ff`), `.rgb` (`173, 198, 255`), `.r`/`.g`/`.b`, а также фильтры `| hex`, `| strip` (`stripped`), `| rgb`, `| hypr_rgb` (`rgb(adc6ff)`), `| hypr_rgba("ee")` (`rgba(adc6ffee)`).

4. **Неразрушающая безопасность**:
   - При `create_backups = true` перед первым изменением `target_config` один раз создаётся эталон `<file>.original`, а перед обновлениями — `<file>.bak`; сгенерированные палитры (`output_path`) пишутся атомарно через `safe_write()` без `.bak`-копий.
   - Импорты изолируются блоком маркеров (`# >>> termonet colors >>> ... # <<< termonet colors <<<`), а старые импорты Pywal аккуратно заменяются на месте.

</details>

---

## ▸ Сборка из исходников и проверки

Требуется Linux (Wayland или X11) и Rust (`edition = "2024"`, `1.85+`):

```bash
git clone https://github.com/Denxrc/termonet.git && cd termonet
cargo build --release
install -Dm755 target/release/termonet ~/.local/bin/termonet
```

<details>
<summary>Проверки качества для контрибьюторов</summary>

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

</details>

---

## ▸ Дорожная карта (Roadmap)

История вышедших релизов (`v0.1.0` – `v1.6.0`) и подробные списки изменений доступны в [GitHub Releases](https://github.com/Denxrc/termonet/releases).

- [ ] **v1.6.x — Расширение экосистемы и стабилизация**
  - [x] Внешний реестр шаблонов ([`Denxrc/termonet-apps`](https://github.com/Denxrc/termonet-apps) + свои `repos/*.toml`, CLI/TUI и еженедельное автообновление)
  - [ ] Новые шаблоны в `termonet-apps` для GTK 3/4 (`gtk.css`), терминалов (`foot`, `alacritty`, `ghostty`), уведомлений (`mako`, `dunst`) и лаунчера `fuzzel`
  - [ ] Живое тестирование и стабилизация бэкендов GNOME, KDE Plasma и X11 (`feh`, `nitrogen`)
- [ ] **v2.0 — Продвинутая колористика, пресеты и визуальный контроль**
  - [ ] Выбор варианта генерации Material 3 (`vibrant`, `expressive`, `tonal-spot`, `content`, `fidelity`, `monochrome`) через конфиг и `--variant`
  - [ ] Настройка контрастности Material 3 (`-1.0`..`1.0`) через `--contrast`
  - [ ] Интерактивный выбор seed-цвета из кандидатов и симулятор синтаксиса/контраста в TUI
  - [ ] Встроенные статические пресеты (Catppuccin Mocha/Latte, Nord, Gruvbox, Tokyo Night) через `--preset <name>`
- [ ] **v3.0+ — Системная экосистема и смелые идеи**
  - [ ] Синхронизация с XDG Desktop Portal (`org.freedesktop.appearance.color-scheme`) через D-Bus
  - [ ] Независимые профили тем для разных мониторов при разных обоях
  - [ ] Экспорт палитр в `base16`, `json` и `css`
  - [ ] *[Экспериментально / В планах]* Порт на Windows (Win32 API обоев, Windows Terminal `settings.json`, PowerShell `$PROFILE`, GlazeWM/Zebar)

---

<div align="center">

<sub>Лицензия MIT · Написано на Rust для Linux · <a href="ARCHITECTURE.ru.md">Архитектура</a> · <a href="README.md">English version</a></sub>

</div>
