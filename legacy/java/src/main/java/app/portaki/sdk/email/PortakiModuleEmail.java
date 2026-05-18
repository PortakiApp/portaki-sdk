package app.portaki.sdk.email;

import java.lang.annotation.Documented;
import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marque une méthode qui compose le contenu d’un e-mail transactionnel. La valeur doit correspondre
 * à {@code emails[].id} du manifeste {@code portaki.module.json}.
 *
 * <p>Signature attendue : {@code Optional<ModuleEmailContent> methodName(StayEmailContext ctx)}.
 *
 * <p>La plateforme invoque la méthode ; le module ne envoie pas l’e-mail lui-même.
 */
@Documented
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface PortakiModuleEmail {

    /** Identifiant stable, aligné sur {@code portaki.module.json → emails[].id}. */
    String value();
}
