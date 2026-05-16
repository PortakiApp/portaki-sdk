package app.portaki.sdk.email;

import java.util.Optional;

/**
 * Contenu fourni par le module pour un envoi. Texte brut uniquement ; la plateforme applique le
 * template HTML.
 */
public record ModuleEmailContent(
        LocalizedText subject, LocalizedText body, Optional<ModuleEmailCta> cta) {

    public ModuleEmailContent {
        if (subject == null) {
            throw new IllegalArgumentException("module_email_subject_null");
        }
        if (body == null) {
            throw new IllegalArgumentException("module_email_body_null");
        }
        cta = cta == null ? Optional.empty() : cta;
    }

    public static ModuleEmailContent of(
            LocalizedText subject, LocalizedText body, ModuleEmailCta cta) {
        return new ModuleEmailContent(subject, body, Optional.of(cta));
    }

    public static ModuleEmailContent of(LocalizedText subject, LocalizedText body) {
        return new ModuleEmailContent(subject, body, Optional.empty());
    }
}
