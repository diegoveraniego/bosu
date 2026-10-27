# Revisión de una versión de Bosu

La compilación y las pruebas de datos no sustituyen la revisión de una app de escritorio.

## Comprobaciones automatizadas

- `cargo fmt --all -- --check`
- `cargo test --locked`
- `cargo clippy --locked -- -D warnings`
- `cargo build --release --locked`
- `desktop-file-validate data/org.diego.Bosu.desktop`
- `appstreamcli validate --no-net data/org.diego.Bosu.metainfo.xml`
- `python scripts/compile-translations.py --check`
- `msgfmt --check po/en.po -o /tmp/bosu-en.mo`
- `BOSU_GIF_TEST_OUTPUT=/tmp/bosu.gif cargo test exports_two_distinct_dated_frames`
- `python scripts/verify_body_gif.py /tmp/bosu.gif`

La prueba GTK está marcada como `ignored` para que las pruebas de datos puedan ejecutarse
sin escritorio. Se ejecuta explícitamente en CI con Xvfb y una sesión D-Bus:

```sh
xvfb-run -a dbus-run-session -- env GTK_A11Y=none G_DEBUG=fatal-warnings \
  BOSU_SCREENSHOT_DIR=/tmp/bosu-screenshots \
  cargo test --locked ui_smoke -- --ignored --test-threads=1
```

Usa datos ficticios en un directorio temporal. Comprueba un guardado parcial, un mapa
vinculado, navegación y recuperación del borrador al cerrar. Genera capturas reales de
la interfaz para revisión; no utiliza datos personales. El workflow `Checks` conserva
las capturas como un artefacto. El workflow `Flatpak bundle`, ejecutable manualmente,
genera un instalador para revisión y no publica nada en Flathub.

## Revisión manual en GNOME

- Abrir desde el lanzador, verificar icono, agrupación de ventanas y activación D-Bus.
- Completar un ejercicio y guardar una sesión parcial; comprobar su actividad semanal.
- Guardar descanso o sensaciones sin ejercicios y verificar su distinción en Progreso.
- Recuperar un borrador después de reiniciar y después de cambiar de rutina.
- Registrar mapas antes y después; comprobar que siguen vinculados al eliminar/deshacer.
- Forzar un fallo de escritura con datos de prueba: no debe aparecer éxito ni vaciarse el formulario.
- Restaurar un respaldo completo y uno inválido; verificar copia de seguridad y conservación de datos.
- Probar diálogos de archivo y exportación GIF dentro de Flatpak mediante el portal.
- Verificar tema claro, oscuro y alto contraste; escalado 100 %, 200 % y ventana estrecha.
- Recorrer controles con Tab/Shift+Tab y operar mapas por nombre, sin arrastrar ni clic derecho.
- Revisar nombres accesibles con Orca; el gráfico tiene valores textuales y el selector temporal anuncia la fecha.
- Activar recordatorios, posponer y desactivar. No se instala un servicio ni se envían avisos con la app cerrada.
- Revisar español e inglés, manteniendo intactos nombres de ejercicios y notas personales.

## Antes de distribuir públicamente

Incluir capturas reales aprobadas en los metadatos de la tienda y revisar la atribución
`src/assets/body-map/NOTICE.md`. El README del proveedor declara MIT, pero su descripción
solicita permiso: esa contradicción debe aclararse con el proveedor antes de una publicación
pública. La licencia del código de Bosu y la del icono original están en `LICENSE`.

No declarar que una versión ha pasado la revisión manual o la compilación Flatpak si esas
comprobaciones no se han ejecutado. Un archivo de configuración para CI no demuestra un
resultado de CI.
