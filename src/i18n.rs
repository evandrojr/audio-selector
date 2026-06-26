use sys_locale::get_locale;

pub struct Translations {
    pub title: &'static str,
    pub tab_devices: &'static str,
    pub advanced_options: &'static str,
    pub hide_unknown_bt: &'static str,
    pub bluetooth: &'static str,
    pub unified: &'static str,
    pub output: &'static str,
    pub input: &'static str,
    pub audio_device: &'static str,
    pub refresh: &'static str,
    pub status_ready: &'static str,
    pub status_applied: &'static str,
    pub status_connecting: &'static str,
    pub filter_active: &'static str,
    pub exclude_instruction: &'static str,
    pub volume: &'static str,
    pub menu_quit: &'static str,
    pub open_logs: &'static str,
}

pub const EN: Translations = Translations {
    title: "Audio Selector", tab_devices: "Devices", advanced_options: "Advanced Options",
    hide_unknown_bt: "Hide unknown Bluetooth devices (MAC addresses)", bluetooth: "Bluetooth",
    unified: "Use same device for input/output", output: "Output Device", input: "Input Device",
    audio_device: "Audio Device", refresh: "Refresh Devices", status_ready: "Ready",
    status_applied: "Applied", status_connecting: "Connecting Bluetooth...",
    filter_active: "Enable Excluded Devices", exclude_instruction: "Check devices below to hide them:",
    volume: "Volume", menu_quit: "Quit", open_logs: "Open Application Logs",
};

pub const PT: Translations = Translations {
    title: "Seletor de Áudio", tab_devices: "Dispositivos", advanced_options: "Opções Avançadas",
    hide_unknown_bt: "Ocultar dispositivos Bluetooth desconhecidos (MACs)", bluetooth: "Bluetooth",
    unified: "Mesmo dispositivo para entrada/saída", output: "Dispositivo de Saída", input: "Dispositivo de Entrada",
    audio_device: "Dispositivo de Áudio", refresh: "Atualizar Dispositivos", status_ready: "Pronto",
    status_applied: "Aplicado", status_connecting: "Conectando Bluetooth...",
    filter_active: "Ativar Dispositivos Excluídos", exclude_instruction: "Marque os dispositivos abaixo para ocultar:",
    volume: "Volume", menu_quit: "Sair", open_logs: "Abrir Logs da Aplicação",
};

pub const ES: Translations = Translations {
    title: "Selector de Audio", tab_devices: "Dispositivos", advanced_options: "Opciones Avanzadas",
    hide_unknown_bt: "Ocultar dispositivos Bluetooth desconocidos (MAC)", bluetooth: "Bluetooth",
    unified: "Mismo dispositivo para entrada/salida", output: "Dispositivo de Salida", input: "Dispositivo de Entrada",
    audio_device: "Dispositivo de Audio", refresh: "Actualizar Dispositivos", status_ready: "Listo",
    status_applied: "Aplicado", status_connecting: "Conectando Bluetooth...",
    filter_active: "Activar Dispositivos Excluidos", exclude_instruction: "Marque los dispositivos a continuación:",
    volume: "Volumen", menu_quit: "Salir", open_logs: "Abrir Logs de la Aplicación",
};

pub const FR: Translations = Translations {
    title: "Sélecteur d'Audio", tab_devices: "Appareils", advanced_options: "Options Avancées",
    hide_unknown_bt: "Masquer les appareils Bluetooth inconnus (MAC)", bluetooth: "Bluetooth",
    unified: "Même appareil pour l'entrée/sortie", output: "Appareil de Sortie", input: "Appareil d'Entrée",
    audio_device: "Appareil Audio", refresh: "Actualiser les Appareils", status_ready: "Prêt",
    status_applied: "Appliqué", status_connecting: "Connexion Bluetooth...",
    filter_active: "Activer Appareils Exclus", exclude_instruction: "Cochez les appareils ci-dessous:",
    volume: "Volume", menu_quit: "Quitter", open_logs: "Ouvrir os Logs de l'Application",
};

pub const DE: Translations = Translations {
    title: "Audio-Selector", tab_devices: "Geräte", advanced_options: "Erweiterte Optionen",
    hide_unknown_bt: "Unbekannte Bluetooth-Geräte ausblenden (MAC)", bluetooth: "Bluetooth",
    unified: "Gleiches Gerät für Ein-/Ausgabe", output: "Ausgabegerät", input: "Eingabegerät",
    audio_device: "Audiogerät", refresh: "Geräte aktualisieren", status_ready: "Bereit",
    status_applied: "Angewendet", status_connecting: "Bluetooth wird verbunden...",
    filter_active: "Ausgeschlossene Geräte aktivieren", exclude_instruction: "Geräte unten ankreuzen:",
    volume: "Lautstärke", menu_quit: "Beenden", open_logs: "Anwendungsprotokolle öffnen",
};

pub const IT: Translations = Translations {
    title: "Selettore Audio", tab_devices: "Dispositivi", advanced_options: "Opzioni Avanzate",
    hide_unknown_bt: "Nascondi dispositivos Bluetooth sconosciuti (MAC)", bluetooth: "Bluetooth",
    unified: "Stesso dispositivo per ingresso/uscita", output: "Dispositivo di Uscita", input: "Dispositivo de Ingresso",
    audio_device: "Dispositivo Audio", refresh: "Aggiorna Dispositivi", status_ready: "Pronto",
    status_applied: "Applicato", status_connecting: "Connessione Bluetooth...",
    filter_active: "Abilita Dispositivi Esclusi", exclude_instruction: "Seleziona i dispositivos qui sotto:",
    volume: "Volume", menu_quit: "Esci", open_logs: "Apri i Log dell'Applicazione",
};

pub fn get_current_translations() -> &'static Translations {
    let loc = get_locale().unwrap_or_else(|| "en".to_string());
    if loc.starts_with("pt") { &PT }
    else if loc.starts_with("es") { &ES }
    else if loc.starts_with("fr") { &FR }
    else if loc.starts_with("de") { &DE }
    else if loc.starts_with("it") { &IT }
    else { &EN }
}
