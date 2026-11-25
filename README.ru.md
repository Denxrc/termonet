<div align="center">

```text
▀█▀ █▀▀▀ █▀▀█ █▀▄▀█ █▀▀█ █▄ █ █▀▀▀ ▀█▀
 █  █▀▀  █▄▄▀ █ █ █ █  █ █ ▀█ █▀▀   █ 
 █  █▄▄▄ █  █ █   █ █▄▄█ █  █ █▄▄▄  █ 
```

# Termonet

**Быстрый и динамический генератор системных тем и палитр Material You для Linux**

[![Rust Edition 2024](https://img.shields.io/badge/rust-edition_2024-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/версия-1.5.0-blue.svg)](Cargo.toml)
[![License: MIT](https://img.shields.io/badge/лицензия-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/платформа-Linux%20(Wayland%20%2F%20X11)-lightgrey.svg)]()
[![XERJ Corpus Hub автор](https://hub.xerj.org/contributors/Denxrc/badge.svg)](https://hub.xerj.org/contributors/Denxrc)

[English](README.md) | [Русский](README.ru.md)

</div>

---

## 🌟 О проекте

> [!NOTE]
> **XERJ Corpus Hub**: Termonet включен в базу [XERJ Corpus Hub](https://hub.xerj.org/contributors/Denxrc) как практический пример реализации TUI, работы с путями и темами на Rust.

**Termonet** — легковесный, безопасный генератор динамических тем оформления и цветовых палитр для графических окружений Linux (Wayland и X11), написанный на **Rust** (edition 2024).

Программа извлекает ключевые цвета из обоев рабочего стола и математически гармонизирует их по алгоритму **Google Material You (Material 3)**, создавая роли цветов и 16-цветную ANSI-палитру терминала. Termonet оснащен удобным терминальным интерфейсом (TUI) на базе `ratatui` с интерактивным мастером настройки, вводом путей к обоям прямо в интерфейсе и безопасным управлением конфигурацией.

---

## ✨ Возможности

- 🎨 **Настоящий Material You (Material 3)**: На базе крейта `material-colors` с использованием алгоритма `Variant::Content`, который сохраняет естественные цвета и приглушенные атмосферные полутона без искусственного завышения насыщенности.
- ⚡ **Мгновенная обработка изображений**: Предварительное масштабирование до 128×128 в памяти через крейт `image`, обеспечивающее расчет палитры всего за ~40 мс для изображений любого разрешения, включая 8K и анимированные GIF-файлы (извлечение первого кадра).
- 🖥️ **Адаптивный TUI с фирменным 5px пиксельным шрифтом**:
  - Пиксельный логотип высотой 5 строк через полублоки Unicode (`█`, `▀`, `▄`) с поддержкой кириллицы и латиницы.
  - Автоматическое переключение на компактный заголовок при малом размере терминала.
- 🧙 **Интерактивный мастер первого запуска**:
  - Полная локализация на **русский** и **английский** языки.
  - Выбор темы оформления TUI (Темная / Светлая / Системная), графического сервера (Wayland / X11) и бэкенда обоев.
  - Вызов в любой момент по клавише `s` или `F2`.
- 🖼️ **Смена обоев «на лету» в интерфейсе**:
  - Нажатие `w` на шаге палитры открывает модальное окно для ввода пути.
  - Автоматическое раскрытие тильды `~` в домашнюю директорию (`~/Pictures/wall.png` ➔ `/home/user/Pictures/wall.png`).
  - Мгновенный пересчет цветов и сохранение в конфиг.
- 📝 **Понятный и комментированный конфиг**:
  - Файл `~/.config/termonet/config.toml` снабжен подробными пояснениями к каждой опции и списком поддерживаемых значений.
- 🛡️ **Безопасность и работа без root-прав**:
  - Работает строго с правами обычного пользователя.
  - Все операции до подтверждения выполняются в оперативной памяти.

---

## 🧭 Навигация и горячие клавиши

Интерфейс разделен на 4 логических шага:

| Шаг | Экран | Описание |
| :---: | :--- | :--- |
| **1** | **Палитра** | Просмотр ролей Material You и 16 цветов ANSI; нажатие `w` открывает ввод пути к обоям |
| **2** | **Программы** | Выбор поддерживаемых приложений (Kitty, Waybar, Rofi, Hyprland) и кастомных целей |
| **3** | **Хуки** | Просмотр и переключение команд перезапуска сервисов (напр. `hyprctl reload`, `killall -SIGUSR2 waybar`) |
| **4** | **Применение** | Сводка обновляемых файлов и запускаемых хуков перед записью |

### Управление

| Клавиша | Действие |
| :--- | :--- |
| `←` / `→` (`h` / `l`) | Переход между окнами (шагами 1..4) |
| `1`, `2`, `3`, `4` | Прямой переход к соответствующему шагу |
| `↑` / `↓` (`k` / `j`) | Навигация по спискам программ и хуков |
| `Пробел` | Включение / отключение выбранной программы или хука |
| `Enter` | Применить тему (на шаге 4) / Переключить опцию или сохранить (в мастере) |
| `Tab` / `m` | Переключение между экранами (на шаге 1, мультимонитор) |
| `w` / `o` | Открыть ввод пути к обоям (на шаге 1) |
| `r` | Автоопределить обои из запущенного демона обоев (на шаге 1) |
| `s` / `F2` | Открыть мастер настроек |
| `Esc` | Закрыть модальное окно ввода / отмена настроек |
| `q` | Выход из Termonet |

---

## 🚀 Установка и сборка

### Требования

- Linux (Wayland или X11)
- Компилятор Rust (1.85+ с поддержкой редакции 2024):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### Сборка из исходников

```bash
git clone https://github.com/Denxrc/termonet.git
cd termonet
cargo build --release
```

Готовый бинарный файл находится в `target/release/termonet`. Для установки в систему:

```bash
install -Dm755 target/release/termonet ~/.local/bin/termonet
```

---

## 💻 Использование

```bash
# Запуск интерактивного TUI (автоматически определит активные обои из демона или конфига)
termonet

# Автономный режим без TUI: применить палитру к конфигам программ из активных обоев и выйти
termonet --apply

# Мгновенное переключение темной/светлой темы без повторного анализа обоев
termonet --toggle-theme

# Явное переключение темы на темную или светлую
termonet --theme dark # или termonet --theme light

# Генерация палитры от конкретного цвета в автономном режиме
termonet -c "#385e72" -a

# Принудительное применение без TUI (игнорируя кэш последних обоев)
termonet --apply --force

# Применение конкретного файла обоев в автономном режиме
termonet --apply -w ~/Pictures/Wallpapers/forest.png

# Запуск TUI с палитрой от конкретного цвета
termonet -c "#385e72"

# Запуск TUI с указанием конкретного изображения обоев
termonet -w ~/Pictures/Wallpapers/forest.png

# Запуск мастера начальной настройки
termonet --init

# Принудительная синхронизация и обновление шаблонов/правил до актуальных
termonet --reset-rules # или termonet --update-rules

# Диагностика: проверка и вывод данных всех поддерживаемых менеджеров обоев
termonet --debug-wallpaper
```

### Параметры командной строки

```text
Использование: termonet [ОПЦИИ] [ПУТЬ_К_ОБОЯМ]

Параметры автономного режима / автоматизации:
  -a, --apply              Автономный режим: применить тему из активных обоев и выйти
  -f, --force              Принудительное применение, игнорируя кэш состояния обоев
  -c, --color <HEX>        Сгенерировать палитру напрямую из HEX-цвета (например, '#385e72')
      --toggle-theme       Мгновенно переключить тему (тёмная/светлая) и применить
      --theme <РЕЖИМ>      Установить и применить тему ('dark' или 'light')
      --no-backup          Отключить создание резервных копий (.bak, .original)
      --backup             Включить создание резервных копий (.bak, .original)

Общие параметры:
  -w, --wallpaper <ПУТЬ>   Путь к изображению обоев (PNG, JPG, GIF, WebP)
  -i, --wizard, --init     Запустить мастер начальной настройки
      --reset-rules, --update-rules  Синхронизировать/сбросить правила и шаблоны по умолчанию
      --debug-wallpaper    Вывести диагностический отчет менеджеров обоев
  -h, --help               Показать справочную информацию
```

---

## ⚙️ Конфигурация

Файл настроек располагается по пути `~/.config/termonet/config.toml` и содержит подробные комментарии:

```toml
# ==============================================================================
# Termonet Configuration
# Путь: ~/.config/termonet/config.toml
# Schema Version: 2
# ==============================================================================

# Язык интерфейса ("ru" | "en")
language = "ru"

# Цветовая тема TUI ("dark" | "light" | "system" по системной схеме рабочего стола)
tui_theme = "dark"

# Режим шрифта заголовка ("pixel" | "standard")
font_mode = "pixel"

# Графический сервер ("wayland" | "x11")
display_server = "wayland"

# Утилита обоев
# Варианты: "auto", "swww", "hyprpaper", "awww", "wpaperd", "swaybg", "gnome", "kde", "feh", "nitrogen", "manual"
wallpaper_backend = "auto"

# Главный монитор (раскомментируйте для указания)
# primary_monitor = "eDP-1"

# Ручной путь к обоям (раскомментируйте для указания)
# manual_wallpaper_path = "~/Pictures/wallpapers/forest.png"

# Программы для генерации цветовых схем
enabled_programs = [
    "kitty",
    "waybar",
    "rofi",
    "hyprland"
]

# Создание резервных копий (false — чистое обновление без мусора .bak)
create_backups = false
```

---

## 🎨 Создание собственных правил и шаблонов тем

Termonet спроектирован полностью модульным: вы можете добавить динамическую поддержку **любой** программы (терминала, бара, лаунчера, редактора кода), просто создав два файла в каталоге `~/.config/termonet/`.

### 1. Архитектура темы

Интеграция каждой программы описывается двумя компонентами:
1. **Файл правил** (`~/.config/termonet/rules/<программа>.toml`) — метаданные, пути к файлам и команда перезапуска.
2. **Файл шаблона** (`~/.config/termonet/templates/<программа>.tmpl`) — разметка конфига с синтаксисом MiniJinja.

### 2. Спецификация файла правил (`rules/<name>.toml`)

```toml
# Версия схемы правила (для автоматической миграции дефолтов)
version = 2

# Имя программы (отображается в TUI и мастере)
name = "Foot"

# Имя файла шаблона в ~/.config/termonet/templates/
template = "foot.tmpl"

# Куда сохранять сгенерированный файл цветов
output_path = "~/.config/foot/colors-termonet.ini"

# (Опционально) Главный конфигурационный файл программы для вставки импорта
target_config = "~/.config/foot/foot.ini"

# (Опционально) Строка импорта, которая автоматически вставится в target_config
import_line = "include = ~/.config/foot/colors-termonet.ini"

# Позиция вставки ("top" - в начало, "bottom" - в конец)
import_position = "bottom"

# (Опционально) Стиль комментариев маркера ("hash" | "blockcss" | "slash" | "doubledash")
# Если не указан, определяется автоматически по расширению файла target_config
comment_style = "hash"

# (Опционально) Команда для мгновенного применения темы без перезапуска программы
reload_cmd = ""
```

### 3. Переменные и фильтры в шаблонах (`templates/<name>.tmpl`)

В шаблонах доступны все роли **Google Material You** (для светлой и темной тем), а также гармонизированная 16-цветная ANSI-палитра терминала.

#### Доступные переменные

- **Динамическая активная тема (`active.*` или `theme.*`)**:
  - Автоматически выбирает свойства светлой или темной темы в зависимости от текущего режима:
  - `active.primary`, `active.on_primary`, `active.primary_container`, `active.on_primary_container`
  - `active.secondary`, `active.on_secondary`, `active.secondary_container`, `active.on_secondary_container`
  - `active.tertiary`, `active.on_tertiary`
  - `active.surface`, `active.on_surface`, `active.surface_variant`, `active.on_surface_variant`
  - `active.background`, `active.on_background`, `active.outline`
  - `active.error`, `active.on_error`
- **Явные варианты схем (`dark.*` и `light.*`)**:
  - Все перечисленные выше роли с префиксами `dark.` или `light.` (включая `error` и `on_error`)
- **Терминальные ANSI-палитры (`terminal.*`, `terminal_dark.*`, `terminal_light.*`)**:
  - `terminal.color0` ... `terminal.color15` (динамически адаптируется под активную тему)
  - `terminal.background`, `terminal.foreground`, `terminal.cursor`
  - `terminal_dark.*` и `terminal_light.*` для явной адресации
- **Ключевой цвет обоев**: `seed_color`

#### Форматы вывода цветов

Каждый цвет предоставляет готовые поля:
- `.hex` — цвет с решеткой: `#adc6ff`
- `.strip` — цвет без решетки: `adc6ff`
- `.rgb` — каналы через запятую: `173, 198, 255`
- `.r`, `.g`, `.b` — отдельные 8-битные каналы (0..255)

#### Встроенные фильтры MiniJinja

- `{{ dark.primary | hex }}` ➔ гарантированный `#adc6ff`
- `{{ dark.primary | strip }}` (или `stripped`) ➔ `adc6ff`
- `{{ dark.primary | rgb }}` ➔ `173, 198, 255`
- `{{ dark.primary | hypr_rgb }}` ➔ `rgb(adc6ff)`
- `{{ dark.primary | hypr_rgba("ee") }}` ➔ `rgba(adc6ffee)`

### 4. Практический пример: Добавление терминала Foot

1. Создайте `~/.config/termonet/rules/foot.toml`:
   ```toml
   version = 2
   name = "Foot"
   template = "foot.tmpl"
   output_path = "~/.config/foot/colors-termonet.ini"
   target_config = "~/.config/foot/foot.ini"
   import_line = "include = ~/.config/foot/colors-termonet.ini"
   import_position = "bottom"
   ```

2. Создайте `~/.config/termonet/templates/foot.tmpl`:
   ```ini
   # Termonet Foot Terminal Theme
   # termonet-version: 4

   [colors]
   background = {{ terminal.background.strip }}
   foreground = {{ terminal.foreground.strip }}
   regular0 = {{ terminal.color0.strip }}
   regular1 = {{ terminal.color1.strip }}
   regular2 = {{ terminal.color2.strip }}
   regular3 = {{ terminal.color3.strip }}
   regular4 = {{ terminal.color4.strip }}
   regular5 = {{ terminal.color5.strip }}
   regular6 = {{ terminal.color6.strip }}
   regular7 = {{ terminal.color7.strip }}
   bright0 = {{ terminal.color8.strip }}
   bright1 = {{ terminal.color9.strip }}
   bright2 = {{ terminal.color10.strip }}
   bright3 = {{ terminal.color11.strip }}
   bright4 = {{ terminal.color12.strip }}
   bright5 = {{ terminal.color13.strip }}
   bright6 = {{ terminal.color14.strip }}
   bright7 = {{ terminal.color15.strip }}
   ```

3. Запустите `termonet` — программа автоматически появится в списке на шаге 2!

### 5. Безопасность и автоматические бэкапы

Termonet заботится о сохранности ваших исходных конфигураций:
- **Эталонный бэкап (`.original`)**: при `create_backups = true` создается строго один раз при первом внедрении в `target_config` и **никогда** не перезаписывается последующими темами.
- **Страховочный бэкап (`.bak`)**: при `create_backups = true` (по умолчанию выключен для чистого обновления) создается перед каждым изменением `target_config`. Сгенерированные файлы палитры (`output_path`) обновляются атомарно без лишних `.bak`-копий.
- **Умные маркеры**: инъекция оформляется блоком комментариев:
  ```ini
  # >>> termonet colors >>>
  include colors-termonet.conf
  # <<< termonet colors <<<
  ```
  Если в конфиге присутствовал старый импорт Pywal, Termonet аккуратно заменяет его на месте, не плодя дубликатов.
- **Принудительное обновление дефолтов**: при обновлении Termonet встроенные шаблоны можно обновить командой:
  ```bash
  termonet --update-rules
  ```

---

## 🗺️ План разработки (Roadmap)

- [x] **v0.0 (Прототип и база)**:
  - [x] Адаптивный TUI на `ratatui` с 5px пиксельным шрифтом
  - [x] Генерация палитры Material You через `material-colors` (`Variant::Content`)
  - [x] Мастер первого запуска и комментированный TOML-конфиг
  - [x] Двуязычный интерфейс (русский и английский)
  - [x] Ручной ввод пути к обоям в TUI и автоподстановка
- [x] **v1.0 (Полноценный релиз для Wayland и X11)**:
  - [x] Модульный шаблонизатор `minijinja` с фильтрами и переменными TrueColor / ANSI
  - [x] Встроенные готовые правила и шаблоны (Kitty, Waybar, Rofi, Hyprland Lua)
  - [x] Безопасная инъекция маркеров, замена legacy-импортов (Pywal) и система вечных `.original` бэкапов
  - [x] Двухуровневое версионирование и автомиграция правил/шаблонов (`--update-rules`, `--reset-rules`)
  - [x] Автоматическое определение обоев в Wayland (`awww`, `swww`, `hyprpaper`, `wpaperd`, `swaybg`) и DE (`GNOME`, `KDE`)
  - [x] Автоматическое определение обоев в X11 (`feh`, `nitrogen`)
  - [x] Мультимониторный фильтр и выбор главного экрана (`primary_monitor`)
  - [x] Автономный CLI-режим (`--apply`, `--force`) с защитой от холостых перезапусков через кэш состояния
- [ ] **v1.5 (CLI-управление темами, расширение шаблонов и стабилизация)**:
  - [x] Мгновенное переключение темной/светлой темы без повторного анализа обоев: `termonet --toggle-theme`, `termonet --theme <dark|light>`
  - [x] Генерация палитры от произвольного цвета через CLI: `termonet --color "#HEX"`
  - [x] Динамический список менеджеров обоев в мастере настройки (разделение Wayland и X11)
  - [ ] Расширение встроенных правил: системный GTK 3/4 (`gtk.css`), терминалы (`foot`, `alacritty`, `ghostty`), уведомления (`mako`, `dunst`), лаунчер `fuzzel`
  - [ ] Практическое тестирование и стабилизация бэкендов окружений (GNOME, KDE Plasma) и X11 (`feh`, `nitrogen`)
- [ ] **v2.0 (Продвинутая колористика, пресеты и визуальный контроль)**:
  - [ ] Поддержка стилей генерации Material 3 (`vibrant`, `expressive`, `tonal-spot`, `content`, `fidelity`, `monochrome`) через конфиг и флаг `--variant`
  - [ ] Настройка контрастности Material 3 (от `-1.0` до `1.0`) для солнечного и ночного режимов (`--contrast`)
  - [ ] Интерактивный выбор акцентного seed-цвета из топ-кандидатов в TUI
  - [ ] Встроенные эталонные статические пресеты (Catppuccin Mocha/Latte, Nord, Gruvbox, Tokyo Night) через `termonet --preset <name>`
  - [ ] TUI-симулятор синтаксиса и контрастности (Mockup Preview) для оценки читаемости кода до сохранения темы
- [ ] **v3.0+ (Системная экосистема и интеграция)**:
  - [ ] Синхронизация с XDG Desktop Portal (`org.freedesktop.appearance.color-scheme`) через DBus (автоматический dark/light mode в браузерах и приложениях)
  - [ ] Независимые профили тем для разных мониторов при различных обоях
  - [ ] Экспорт палитр в стандартные форматы (`base16`, `json`, `css`)
  - [ ] *[Экспериментально / В планах]* Кроссплатформенный порт на Windows (Win32 API обоев, Windows Terminal `settings.json`, PowerShell `$PROFILE`, GlazeWM/Zebar)

---

## 📄 Лицензия

Проект распространяется под открытой лицензией [MIT License](LICENSE).
