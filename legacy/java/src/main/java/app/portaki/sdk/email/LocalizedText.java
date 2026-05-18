package app.portaki.sdk.email;

/**
 * Libellé bilingue (corps d’e-mail, sujet, bouton). La plateforme choisit la variante selon la locale
 * invité.
 */
public record LocalizedText(String fr, String en) {

    public LocalizedText {
        if (fr == null || fr.isBlank()) {
            throw new IllegalArgumentException("localized_text_fr_blank");
        }
        if (en == null || en.isBlank()) {
            throw new IllegalArgumentException("localized_text_en_blank");
        }
    }

    public static LocalizedText of(String fr, String en) {
        return new LocalizedText(fr, en);
    }

    public String forLocale(String lang) {
        if (lang != null && lang.equalsIgnoreCase("en")) {
            return en;
        }
        return fr;
    }
}
