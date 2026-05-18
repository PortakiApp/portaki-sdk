package app.portaki.sdk.email;

/** Bouton principal de l’e-mail (libellé + action invité). */
public record ModuleEmailCta(LocalizedText label, ModuleGuestEmailAction action) {

    public ModuleEmailCta {
        if (label == null) {
            throw new IllegalArgumentException("module_email_cta_label_null");
        }
        if (action == null) {
            throw new IllegalArgumentException("module_email_cta_action_null");
        }
    }

    public static ModuleEmailCta of(LocalizedText label, ModuleGuestEmailAction action) {
        return new ModuleEmailCta(label, action);
    }
}
