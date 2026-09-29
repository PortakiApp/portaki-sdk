# Inventaire des primitives du livret voyageur

Document de travail pour la conception graphique des primitives SDUI **côté voyageur** — le
livret que voit la personne qui séjourne dans le logement. Le chantier des écrans **hôte**
(tableau de bord) est mené ailleurs ; quand une primitive sert aux deux, c'est dit.

Arrêté au 29 septembre 2026, sur les versions publiées (`origin/main`) de `portaki-sdk`,
`portaki-guest` et `portaki-modules`.

---

## 1. Ce qu'il faut savoir avant de lire

**Une primitive, c'est quoi.** Un module (la météo, le guide d'accès, le règlement…) ne dessine
rien lui-même. Il décrit ce qu'il veut afficher sous forme d'un arbre de blocs nommés —
« une carte, contenant un empilement, contenant un texte et un bouton ». Le livret, lui, sait
dessiner chacun de ces blocs. Ces blocs sont les **primitives**. Il y en a **116** au catalogue.

Conséquence directe pour la conception : dessiner une primitive, c'est fixer l'apparence de
**tous** les modules qui s'en servent, présents et à venir. Il n'y a pas de rattrapage par module.

**Trois réglages communs à toutes les primitives.** Chacune des 116 accepte, en plus de ses
champs propres :

| Réglage | Ce qu'il dit | Valeurs |
|---|---|---|
| `tone` | le rôle de la couleur | 9 valeurs — voir §4 |
| `emphasis` | l'appui du texte | `subtle` · `default` · `strong` |
| `surface` | le niveau du fond | `default` · `elevated` · `sunken` |
| `animation` | l'entrée / la sortie | `fadeUp` · `fadeIn` · `scaleIn` · `slideRight` · `none` |
| `visibility` | une condition d'affichage | (expression, pas une apparence) |

Ces cinq-là existent partout, mais ne **font** pas quelque chose partout : voir §5.

**Deux mots du vocabulaire du dépôt, traduits une fois pour toutes :**

- une **surface**, c'est un écran ou un bloc qu'un module fournit au livret. Il y en a cinq
  sortes côté voyageur : `home.card` (la carte du module sur l'accueil), `explore.detail` /
  `explore.sheet` / `explore.forecast` (la page détaillée), `upcoming.card` (la carte avant
  l'arrivée), `post-stay.card` (après le départ) et `guest.form` (un formulaire) ;
- un **module**, c'est une fonctionnalité installable. Il y en a 21 aujourd'hui.

---

## 2. Les chiffres, et comment je les ai obtenus

| | |
|---|---|
| Primitives au catalogue | **116** |
| Que le livret sait dessiner | **92** — les 24 autres afficheraient un encadré « Unknown primitive » |
| Réellement employées par au moins un module côté voyageur | **35** |
| Employées côté hôte | 36 |
| Employées des deux côtés | 18 |
| **Employées par personne, nulle part** | **63** |
| Surfaces voyageur existantes | 45, réparties sur 19 des 21 modules |

Les deux modules sans aucune surface voyageur sont `ical-sync` et `nuki` : ils travaillent en
arrière-plan et ne parlent qu'à l'hôte.

### Comment j'ai compté

Deux relevés indépendants, puis l'union des deux.

1. **Le code des modules.** Pour chacun des 21 modules, j'ai cherché dans tout son code source
   les appels à chaque primitive, en séparant les fichiers du dossier « voyageur » de ceux du
   dossier « hôte ». Le code partagé entre les deux (par exemple la table qui associe un temps
   qu'il fait à une icône, chez la météo) est compté du côté que le module sert réellement.
2. **Les aperçus livrés.** 17 modules publient un fichier d'aperçu qui contient l'arbre
   **déjà fabriqué** de 28 de leurs surfaces voyageur. Je l'ai parcouru en relevant chaque bloc
   et chaque valeur de variante. C'est le relevé le plus sûr : c'est exactement ce qui part vers
   le livret.

Le deuxième relevé a rattrapé ce que le premier manquait : des valeurs choisies au moment de
l'exécution plutôt qu'écrites en clair (les teintes des bacs de tri chez `waste-recycling`,
l'icône `train` chez `train`). Le premier a rattrapé ce que le second manquait : les 17 surfaces
sans aperçu publié.

### Ce que mon comptage peut manquer

- **Je n'ai pas exécuté les modules.** Un module pourrait construire un bloc par un chemin de
  code que ni le relevé textuel ni les aperçus ne montrent. C'est peu probable — les deux
  relevés concordent — mais ce n'est pas exclu.
- **Les 17 surfaces sans aperçu** ne sont couvertes que par le relevé textuel, moins précis sur
  les valeurs de variante.
- **Le comptage se fait par module, pas par occurrence.** « Employée par 9 modules » ne dit pas
  si elle apparaît une fois ou cinquante fois dans chacun.
- **Le livret lui-même fabrique des arbres de démonstration** (le mode démo, sans séjour réel).
  Ils emploient quelques primitives que nul module n'emploie — `Hero`, `HeaderTitle`,
  `ChecklistItem`, `ProgressBar`. Je ne les ai **pas** comptées comme « employées » : ce sont des
  données de vitrine, pas un usage produit. Elles se verraient pourtant dans une démo.
- Je n'ai regardé que les 21 modules officiels. Un module communautaire futur pourrait se servir
  de n'importe laquelle des 116.

---

## 3. Les primitives, par famille

Dans les tableaux :

- **Employée** = employée aujourd'hui par au moins un module, côté voyageur, avec le nombre de
  modules et un exemple concret (module + surface).
- **hôte aussi** signale qu'elle sert également au tableau de bord de l'hôte.
- **non dessinée** signale que le livret n'a aucun rendu pour elle : si un module l'envoyait, le
  voyageur verrait un encadré pointillé « Unknown primitive ».

> **Lecture rapide pour la conception.** Les lignes marquées **employée** méritent un dessin
> abouti. Les lignes **jamais employée** peuvent attendre, ou disparaître (§5). Les lignes
> **non dessinée** ne concernent pas le livret pour l'instant.

### 3.1 Contenants et mise en page — *empiler, aligner, espacer, sans rien afficher en propre*

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `Stack` | empile ses blocs, verticalement ou horizontalement, avec un écart réglable | `direction` : `vertical` · `horizontal` | **oui — 18 modules** (ex. `access-guide` / `explore.detail`) |
| `Grid` | range ses blocs en colonnes régulières | nombre de colonnes, écart, largeur minimale | **oui — 1** (`weather` / `explore.forecast`, la table des prévisions) |
| `Group` | regroupe sans rien changer à la disposition | — | non |
| `Split` | deux blocs côte à côte, avec une proportion | proportion | non |
| `Indent` | décale un bloc vers la droite | 6 niveaux, 12 px chacun | non |
| `Spacer` | un vide d'une hauteur donnée | hauteur | non |
| `Divider` | un trait de séparation | — | **oui — 2** (`weather` / `explore.forecast`) |
| `SafeArea` | protège des bords de l'écran (encoche, barre du bas) | bords à protéger | non |
| `Anchor` | pose un point d'ancrage dans la page | — | non |
| `PullToRefresh` | devrait permettre de tirer pour rafraîchir | — | non — **et le livret n'en fait rien** : il affiche simplement son contenu |

### 3.2 Blocs de page — *ce qui encadre un contenu et lui donne un titre*

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `Card` | **la pièce maîtresse du livret.** Un encadré au coin arrondi, avec en tête une icône, un titre, un sous-titre, et un « Voir › » s'il mène quelque part | icône, action ; **trois rendus différents**, voir plus bas | **oui — 19 modules**, tous (ex. `access-guide` / `upcoming.card`) · hôte aussi |
| `Section` | un titre + sous-titre suivis de leur contenu | — | non · *(ne pas confondre avec le module `sections`)* |
| `Surface` | un simple bloc avec sa marge intérieure | — | non |
| `Hero` | une bannière d'ouverture, titre sur illustration | 5 illustrations nommées (méditerranée, forêt, crique, ville, campagne) | non — sauf dans la démo du livret |
| `Accordion` | des volets qu'on déplie un à un | — | non (hôte : 1) |
| `Tabs` | des onglets | — | non — **et le rendu est incomplet** : les onglets ne sont pas cliquables, seul le contenu du premier s'affiche |
| `Page` | l'enveloppe d'un écran plein | — | non — **non dessinée** côté voyageur ; c'est l'enveloppe des écrans hôte (20 modules) |

> **`Card` a trois rendus, et c'est le point le plus important du document.** Selon l'endroit :
> 1. **la carte pleine** — encadré, ombre douce, en-tête icône/titre/sous-titre, contenu dessous ;
> 2. **la rangée compacte** — dans une liste d'onglets, la carte se replie en une simple ligne
>    (icône carrée, libellé, accroche, chevron) qui ouvre le contenu ; ses enfants ne sont
>    **pas** affichés à cet endroit ;
> 3. **la bannière de formalités d'arrivée** — un rendu dédié quand la carte contient le bloc
>    de déclaration de séjour.
>
> Le même objet, envoyé par le même module, prend donc trois apparences. Toute maquette de
> `Card` doit couvrir les trois, ou dire laquelle elle fixe.

### 3.3 Fenêtres et couches — *ce qui s'ouvre par-dessus*

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `Modal` | une fenêtre par-dessus la page | — | non — **non dessinée** |
| `BottomSheet` | un panneau qui monte du bas | points d'accroche | non — **non dessinée** |
| `ConfirmDialog` | une demande de confirmation | actions confirmer / annuler | non — **non dessinée** |
| `PreviewPane` | un panneau d'aperçu | — | non — **non dessinée** (outil de l'espace développeur) |

> **Attention, piège de vocabulaire.** Les panneaux du livret existent bel et bien — le voyageur
> en ouvre en permanence — mais ils ne passent **pas** par ces primitives. Un module demande
> l'ouverture par une *action* (`openOverlay`), en précisant `modal`, `bottomSheet` ou
> `fullscreen` ; le livret possède son propre panneau et y place la surface demandée. Les quatre
> primitives ci-dessus sont, en pratique, sans emploi. Le panneau réel, lui, mérite une maquette :
> il a un en-tête (icône dans une pastille, titre, croix de fermeture), et c'est le seul rendu de
> panneau du livret. **Aujourd'hui, `fullscreen` est la seule présentation employée** (5 modules).

### 3.4 Texte — *les mots*

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `Text` | **le bloc de texte de base** | `variant` : `body` · `caption` · `title` · `display` | **oui — 19 modules**, tous (ex. `access-guide` / `explore.detail`) |
| `RichText` | du texte mis en forme par l'hôte (gras, listes, liens) | — | **oui — 2** (`appliances` / `explore.item`) |
| `Markdown` | du texte au format Markdown | — | **oui — 1** (`sections` / `home.card`) |
| `Eyebrow` | une sur-titre court, en capitales | — | **oui — 1** (`appliances` / `explore.item`) · hôte aussi |
| `Quote` | une citation, barre verticale et italique | attribution | non |
| `Highlight` | un surlignage | — | non |
| `Code` | du texte à chasse fixe sur fond gris | langage | non |

> **`Text` porte quatre tailles, mais `display` n'est pas une taille.** Le rendu actuel : `body`
> = texte courant ; `caption` = 12,5 px, gris ; `title` = 19 px dans la police de titrage ;
> `display` = **une pastille carrée de 54 px** destinée à recevoir un emoji ou un symbole — sauf
> si le texte ressemble à une température (`23 °`), auquel cas il devient un grand nombre de
> 30 px. Deux rendus sans rapport sous un même nom, départagés par la **forme du texte**.
> Voir §5.

### 3.5 Listes et rangées — *des lignes successives séparées d'un filet*

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `ListItem` | **la rangée standard** : visuel à gauche, titre, sous-titre, chevron à droite | icône ou emoji en tête, action, chevron | **oui — 12 modules** (ex. `access-guide` / `explore.detail`) · hôte aussi |
| `ColorDotItem` | une rangée précédée d'une pastille de couleur | `swatch` : 10 teintes nommées | **oui — 1** (`waste-recycling` / `explore.detail`, les bacs de tri) |
| `TimedEntry` | une rangée horaire : heure à gauche, libellé, mention à droite | — | **oui — 1** (`train` / `explore.detail`) |
| `List` | l'enveloppe d'une suite de rangées, avec les filets | — | non (hôte : 4) |
| `ChecklistItem` | une rangée à cocher | coché / non coché | non (hôte : 2) — employée dans la démo du livret |
| `SectionListItem` | un intitulé de sous-groupe dans une liste | — | non — **et son rendu est cassé** : il affiche deux fois le même titre (voir §5) |
| `BulletList` | une liste à puces | — | non |
| `IndexedInput` | une rangée numérotée avec un champ | — | non — **non dessinée** |

> **`ListItem` porte trois automatismes** que la conception doit connaître : si le visuel en tête
> ressemble à un nom d'icône, c'est une icône dans une pastille carrée ; sinon, c'est un emoji
> dans une pastille un peu plus grande. Et si le **titre est un nombre seul** avec un
> sous-titre, il devient une pastille ronde numérotée et le sous-titre prend la place du titre —
> c'est ainsi que sont dessinées les suites d'étapes.

### 3.6 Actions — *ce sur quoi on appuie*

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `Button` | le bouton, pleine largeur, 42 px, entièrement arrondi | `variant` : `filled` · `outline` · `ghost` | **oui — 9 modules** (ex. `access-guide` / `explore.detail`) · hôte aussi |
| `Link` | un lien souligné dans la couleur principale | — | **oui — 6** (`appliances` / `explore.item`) |
| `Pressable` | rend n'importe quel bloc cliquable, sans apparence propre | — | **oui — 4** (`emergency-contacts` / `explore.detail`) |
| `EmergencyButton` | un bouton pleine largeur en rouge d'urgence | — | non |
| `IconButton` | un bouton carré de 40 px portant une icône | icône | non — **et son rendu écrit le nom de l'icône en toutes lettres** au lieu de la dessiner |
| `ActionRow` | une rangée de petits boutons | liste d'actions | non |
| `BackButton` | un « ← Retour » | — | non — **et il ne peut rien faire** : le contrat ne lui donne aucune action |

### 3.7 Saisie — *les formulaires du voyageur*

Cinq modules font remplir quelque chose au voyageur : `consumables`, `guest-reviews`,
`issue-report`, `lost-found`, `pre-arrival-form`.

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `Form` | le formulaire, et ce qu'il envoie à la validation | action d'envoi | **oui — 5** (`consumables` / `guest.form`) · hôte aussi |
| `Field` | l'étiquette au-dessus d'un champ, avec l'astérisque si obligatoire | obligatoire ou non | **oui — 5** · hôte aussi |
| `TextInput` | une ligne de saisie, 40 px, coin 10 px | — | **oui — 3** (`issue-report` / `guest.form`) · hôte aussi |
| `TextArea` | une zone de saisie multiligne | nombre de lignes | **oui — 5** · hôte aussi |
| `Select` | une liste déroulante | options | **oui — 1** (`guest-reviews` / `post-stay.card`) · hôte aussi |
| `ChoiceList` | un choix parmi plusieurs | `layout` : `compact` · `cards` | **oui — 3** (`consumables` / `guest.form`) · hôte aussi |
| `TimePicker` | une heure | — | **oui — 1** (`pre-arrival-form` / `guest.form`) |
| `ImageUpload` | l'envoi d'une photo, avec aperçu et retrait | — | **oui — 1** (`issue-report` / `guest.form`) |
| `RadioGroup` | des boutons radio | options | non |
| `Checkbox` | une case à cocher | — | non |
| `Toggle` | une case à cocher avec libellé | — | non (hôte : 1) |
| `NumberInput` | un champ numérique | min, max | non |
| `SearchInput` | un champ de recherche | — | non |
| `Slider` | un curseur | min, max | non |
| `DatePicker` | une date | — | non |
| `TimeSlotPicker` | un choix de créneaux horaires | créneaux | non — **et les créneaux ne sont pas sélectionnables** |
| `TagInput` | des étiquettes qu'on ajoute | — | non — **et son invite est en anglais en dur** |
| `Chips` | des étiquettes en lecture seule | — | non |
| `FormStepper` | l'avancement dans un formulaire en étapes | nombre, étape courante | non |
| `FieldHint` | une aide sous un champ | — | non — **non dessinée** (hôte : 5) |
| `SecretInput` | un champ masqué | — | non — **non dessinée** (hôte : 4) |
| `SelectableCard` | une carte qu'on choisit | icône, action | non — **non dessinée** (hôte : 1) |
| `ToggleRow` | une rangée avec interrupteur | icône, action | non — **non dessinée** (hôte : 5) |
| `EditableList` | une liste d'éléments qu'on modifie | bilingue, photo, case | non — **non dessinée** (hôte : 1) |
| `StepList` | des étapes qu'on ajoute et retire | action d'ajout | non — **non dessinée** (hôte : 3) |
| `RichTextEditor` | un éditeur de texte mis en forme | — | non — **non dessinée** (hôte : 4) |
| `AddressMapPicker` | une adresse choisie sur une carte | — | non — **non dessinée** (hôte : 2) |
| `MapEditor` | des repères qu'on pose sur une carte | — | non — **non dessinée** |
| `CredentialField` | une clé d'accès à un service | — | non — **non dessinée** |
| `InlineNotice` | un message dans un formulaire | — | non — **non dessinée** (hôte : 1) |

> **Le lot « saisie » est très déséquilibré.** Huit primitives sont employées côté voyageur ;
> les quatorze autres sont soit du matériel d'hôte, soit rien du tout. Et **les huit employées
> sont, pour l'essentiel, des contrôles natifs du navigateur** habillés d'une bordure et d'un
> coin arrondi. Il y a là un vrai chantier de conception, mais qui ne porte que sur ces huit.

### 3.8 Filtres

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `FilterChip` | une pastille de filtre, pleine quand elle est active | sélectionnée, action | **oui — 1** (`train` / `explore.detail`) |
| `FilterBar` | la rangée de filtres | liste de filtres | **oui — 1** (`train` / `explore.detail`) |

> `FilterBar` a deux rendus : s'il contient des `FilterChip`, il les aligne et ils sont
> cliquables ; s'il ne porte qu'une liste de libellés, il les affiche en pastilles grises
> **inertes**. Seul le premier chemin est employé.

### 3.9 Médias et repères — *les images et la carte*

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `Map` | une carte Mapbox avec des repères, regroupés ou non | `interactionMode` : `pan-zoom` · `none` ; repères : `property` · `poi` | **oui — 3** (`access-guide`, `events`, `local-guide`) |
| `Icon` | une icône seule, taille réglable | `name` : ~83 jetons — voir §4 | **oui — 1** (`weather` / `explore.forecast`) |
| `Image` | une image pleine largeur au coin arrondi, avec flou de chargement | `size` : `full` · `thumb` (**ignoré par le livret**) | **oui — 1** (`local-guide`) · hôte aussi |
| `QRCode` | censé afficher un code QR | taille | **oui — 1** (`guest-reviews`) — **mais le rendu actuel est un faux motif en dur**, illisible par un téléphone (voir §5) |
| `Avatar` | une photo ronde, ou des initiales | — | non |

> En **aperçu** (espace développeur, vitrine du catalogue), la carte n'est pas affichée : un
> rectangle gris indique seulement « N repères — non affichée en aperçu ». C'est normal, et cela
> explique l'absence de carte sur les captures d'aperçu.

### 3.10 Étiquettes et valeurs — *les petits éléments qui qualifient*

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `KeyValue` | une ligne « intitulé … valeur », valeur à droite | affichage à chasse fixe | **oui — 5** (`access-guide` / `explore.detail` — codes, horaires, mots de passe) |
| `Pill` | une pastille arrondie : point coloré + libellé | teinte du rôle | **oui — 2** (`local-guide` / `explore.detail`) · hôte aussi |
| `Badge` | une pastille arrondie sans le point | teinte du rôle | **oui — 1** (`access-guide` / `explore.detail`) |
| `Temperature` | une température | `variant` : `inline` · `hero` · `compact` ; `unit` : `C` · `F` | **oui — 1** (`weather` / `explore.forecast`) |
| `Tag` | un mot gris, sans fond | — | non — **et il ignore complètement `tone`** |
| `Dot` | un point de couleur, éventuellement clignotant | `swatch` ou `tone` | non |
| `StatusBadge` | une pastille « libellé · état » | — | non — **et l'état s'affiche non traduit** |
| `Stat` | un grand chiffre avec son intitulé et son évolution | icône, `deltaTone` | non — **non dessinée** (hôte : 6) |
| `CountdownTimer` | censé décompter jusqu'à une date | — | non — **et il affiche la date brute, sans décompte** |
| `WeatherIcon` | censé dessiner un temps qu'il fait | condition | non — **et il affiche toujours le même « ☀ »**, quel que soit le temps. Le code le dit lui-même hérité ; `Icon` le remplace |
| `TimeColumn` | une heure au-dessus d'un intitulé | — | non |
| `DateColumn` | une date au-dessus d'un intitulé | — | non |

### 3.11 Données en volume

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `Chart` | un graphique | `kind` : `bars` · `horizontal_bars` · `donut` · `heatmap` | non — **non dessinée** (hôte : 5) |
| `DataTable` | un tableau de données | — | non — **non dessinée** |
| `Timeline` | une frise d'événements | — | non — **non dessinée** |
| `FeedItem` | une entrée de journal, avec état et point coloré | `dotTone`, action | non — **non dessinée** (hôte : 5) |

> Ces quatre-là sont clairement du matériel de tableau de bord. Rien à dessiner pour le livret.

### 3.12 Retours d'état — *dire qu'il n'y a rien, que ça charge, que c'est fait*

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `EmptyState` | « il n'y a rien ici » : titre, description, icône, centré | icône | **oui — 14 modules** · hôte aussi |
| `InfoBanner` | l'encart d'information : icône, titre, message, fond teinté doux | `tone` (partiellement) | **oui — 9 modules** (ex. `access-guide` / `explore.detail`) · hôte aussi |
| `Notice` | une simple ligne grise | — | non |
| `ErrorState` | un encadré rouge : titre, message, et normalement un bouton « réessayer » | action de reprise (**ignorée**) | non |
| `SuccessState` | un encadré vert : titre, message | — | non |
| `CompletionState` | une simple ligne verte en gras | — | non |
| `LoadingState` | un rond qui tourne + un message | — | non |
| `Spinner` | un rond qui tourne, seul | — | non |
| `Skeleton` | un rectangle gris pulsant, en attendant le contenu | `variant`, `lines` (**tous deux ignorés**) ; hauteur, largeur | non |
| `ProgressBar` | une barre de progression | valeur, maximum | non — employée dans la démo du livret |
| `Stepper` | une rangée de segments, ceux franchis en couleur | nombre, courant | non |
| `DotIndicator` | des points de pagination | nombre, courant | non |
| `Toast` | un message éphémère | — | non — **et le livret n'affiche rien du tout** pour cette primitive |

> **Deux pièges sur `EmptyState`.** D'abord, **il est très souvent invisible** : quand le vide
> vient d'une absence de configuration (« l'hôte n'a rien saisi »), le livret n'affiche rien
> plutôt qu'un message. Seuls les vides dus à une erreur s'affichent. Ensuite, quand une `Card`
> ne contient qu'un `EmptyState` de ce genre, **toute la carte disparaît**. C'est la bonne
> décision produit ; il faut simplement savoir que la maquette d'un `EmptyState` sera rarement vue.

### 3.13 Habillage de l'application

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `TopBar` | une barre de titre en haut | — | non |
| `HeaderTitle` | un grand titre avec sous-titre | — | non — employée dans la démo du livret |
| `BottomTabBar` | une barre d'onglets en bas | onglets, onglet actif | non |
| `Chevron` | un chevron `›` | `direction` (**ignoré**) | non |

> L'habillage réel du livret — en-tête, navigation, onglets — est dessiné par le livret lui-même,
> pas par ces primitives. Aucun module n'en envoie, et il est peu probable qu'aucun le fasse :
> un module n'a pas à décider de la barre de navigation. **Bon candidat au retrait** (§5).

### 3.14 Ce qui appartient à la plateforme

| Primitive | À quoi elle sert | Variantes | Employée côté voyageur |
|---|---|---|---|
| `HostFragment` | un bloc que le **livret** dessine, qu'un module se contente d'appeler | identifiant du bloc | **oui — 1** (`pre-arrival-form` / `home.card`) |
| `CapabilityNotice` | « cette fonction n'est pas disponible » | — | non — **non dessinée** |
| `QuotaIndicator` | « vous avez utilisé N sur M » | — | non — **non dessinée** |

> `HostFragment` n'a **qu'une seule** réalisation aujourd'hui : la ligne de déclaration de séjour
> (formalités d'arrivée). Tout autre identifiant n'affiche rien. C'est le mécanisme par lequel un
> sujet sensible — l'identité du voyageur — reste dessiné et servi par la plateforme, jamais par
> le module.

---

## 4. Les variantes : lesquelles servent, lesquelles dorment

### 4.1 `Tone` — 9 valeurs, 5 employées côté voyageur

| Valeur | Voyageur | Hôte | Ce qu'elle donne dans le livret |
|---|---|---|---|
| `neutral` | — | 3 | fond de surface, encre par défaut |
| `primary` | 1 (`pre-arrival-form`) | 3 | fond dans la couleur principale, encre contrastée |
| `secondary` | **jamais** | **jamais** | un mélange de la principale et de l'encre |
| `accent` | **jamais** | **jamais** | le jaune d'accentuation |
| `info` | 1 (`weather`) | 1 | — |
| `success` | 2 (`pre-arrival-form`, `weather`) | 5 | — |
| `warning` | 1 (`weather`) | 10 | — |
| `danger` | 1 (`weather`) | 3 | — |
| `emergency` | **jamais** | **jamais** | fond rouge plein |

**Le jeu de tons est à revoir, et voici pourquoi précisément.** Trois valeurs sur neuf ne sont
employées nulle part (`secondary`, `accent`, `emergency`). Mais le problème le plus sérieux est
ailleurs : **les tons ne se comportent pas pareil selon qu'ils colorent un fond ou une encre.**

| Ton | En **fond** | En **encre / point** |
|---|---|---|
| `info` | surface surélevée + liseré dans la couleur principale | la couleur *info* |
| `success` | la couleur **principale** à 12 % | la couleur *success* (verte) |
| `warning` | la couleur **d'accentuation** à 35 % | la couleur *warning* (orangée) |
| `danger` | le rouge d'urgence à 15 % | le rouge d'urgence |
| `emergency` | le rouge d'urgence, plein | le rouge d'urgence |

Autrement dit : un bloc « succès » **en fond** prend la teinte de la marque, pas du vert — alors
que le même mot « succès » **en texte** est vert. Et `danger` et `emergency` donnent exactement
la même encre : ils ne se distinguent que par l'intensité du fond. Un designer qui reçoit neuf
noms de tons croira à neuf couleurs ; il y en a moins, et elles ne sont pas stables d'un usage
à l'autre.

### 4.2 `TextVariant` — 4 valeurs, 4 employées

| Valeur | Voyageur | Rendu |
|---|---|---|
| `caption` | **18 modules** | 12,5 px, gris à 55 % |
| `body` | **12 modules** | texte courant |
| `title` | 5 modules | 19 px, police de titrage |
| `display` | 1 module (`appliances`) | pastille emoji de 54 px, **ou** grand nombre si le texte ressemble à une température |

Jeu sain — c'est le seul qui soit à la fois entièrement employé et à peu près cohérent. Seul
`display` pose problème (§5).

### 4.3 `ButtonVariant` — 3 valeurs, 1 employée côté voyageur

| Valeur | Voyageur | Hôte |
|---|---|---|
| `filled` | **jamais explicitement** — mais c'est la valeur par défaut du livret, donc tout bouton sans variante est plein | — |
| `outline` | 4 modules | 4 |
| `ghost` | **jamais** | 2 |

Le seul bouton qu'un module choisit délibérément côté voyageur est le bouton **contour**. Le
bouton plein n'est jamais demandé : il arrive par défaut.

### 4.4 `Swatch` — 10 teintes nommées, 4 employées côté voyageur

| Employées côté voyageur | `yellow`, `green`, `brown`, `grey` — toutes par `waste-recycling`, pour les couleurs des bacs de tri |
|---|---|
| Employées côté hôte seulement | `blue`, `black`, `red`, `orange` |
| **Jamais employées** | `white`, `purple` |

`Swatch` a un rôle clair et différent de `Tone` : ce sont des couleurs **du monde réel** (un bac
jaune est jaune), pas des rôles d'interface. La distinction est bonne. Deux teintes dorment.

### 4.5 `IconName` — 83 jetons, 36 employés côté voyageur

**Employés côté voyageur (36)** — `calendar`, `car`, `check-circle`, `circle-x`, `clipboard`,
`clipboard-list`, `clock`, `clock-circle`, `cloud`, `cloud-fog`, `cloud-lightning`, `cloud-rain`,
`cloud-snow`, `cloud-sun`, `danger-triangle`, `droplets`, `gauge`, `home`, `key`, `list-checks`,
`logout`, `map-pin`, `message-circle`, `package`, `package-search`, `phone`, `plug`, `recycle`,
`scale`, `search`, `search-x`, `sparkles`, `star`, `sun`, `thermometer`, `train`, `volume-2`,
`wifi`, `wind`, `zap`.

**Employés côté hôte seulement (16)** — `bell`, `building`, `gift`, `grid`, `image`,
`info-circle`, `link`, `lock`, `mail`, `message`, `more-horizontal`, `plus`, `refresh`, `smile`,
`ticket`, `users`.

**Jamais employés, nulle part (27)** — `ban`, `check`, `chevron-right`, `cloud-off`, `dots`,
`file-text`, `fingerprint`, `guests`, `handshake`, `heart-handshake`, `info`, `list`, `minus`,
`no`, `noise`, `ok`, `parking`, `paw`, `paw-print`, `pets`, `quiet`, `send`, `sliders`,
`triangle-alert`, `user`, `volume-x`, `x`.

**Et surtout : 17 glyphes sont déjà partagés par plusieurs jetons.** Le livret dessine donc la
même chose sous des noms différents :

| Un seul dessin | …pour ces jetons |
|---|---|
| horloge | `clock-circle`, `clock`, `quiet` |
| liste cochée | `clipboard-list`, `list`, `list-checks` |
| interdiction | `minus`, `ban`, `volume-x` |
| triangle d'alerte | `triangle-alert`, `danger-triangle` |
| coche cerclée | `check-circle`, `ok` |
| croix | `x`, `no` |
| information cerclée | `info-circle`, `info` |
| voiture | `car`, `parking` |
| colis | `package`, `package-search` |
| cœur | `paw`, `pets` |
| personnes | `users`, `guests` |
| poignée de main | `handshake`, `heart-handshake` |
| bulle | `message`, `message-circle` |
| haut-parleur | `noise`, `volume-2` |
| points de suspension | `more-horizontal`, `dots` |
| presse-papier | `clipboard`, `file-text` |
| **cadeau** | `gift`, **`paw-print`** |

La dernière ligne n'est sûrement pas voulue : **`paw-print` dessine un cadeau**. Par ailleurs
`sliders` dessine un engrenage et `gauge` dessine un graphique d'activité — deux replis assumés
dans le code, mais qui ne correspondent pas au nom.

Au total : 83 noms, 36 employés, et le livret ne sait en dessiner qu'environ 66 distincts.

### 4.6 Les jeux entièrement inemployés

| Jeu | Valeurs | Constat |
|---|---|---|
| `AnimationKind` | `fadeUp` · `fadeIn` · `scaleIn` · `slideRight` · `none` | **aucun module ne demande jamais d'animation.** Le livret anime pourtant tout `Stack` en `fadeUp` par défaut, et seul `Stack` est animé : `fadeIn`, `scaleIn` et `slideRight` ne sont dessinés nulle part, `slideRight` n'est même pas pris en charge |
| `DeltaTone` | `good` · `bad` · `neutral` | jamais employé (dépend de `Stat`, primitive d'hôte) |
| `Emphasis` | `subtle` · `default` · `strong` | `subtle` (2 modules) et `strong` (1) ; **`default` jamais demandé** — c'est le défaut implicite |
| `SurfaceLevel` | `default` · `elevated` · `sunken` | `elevated` seulement (3 modules) ; `default` et `sunken` **jamais** |
| `TempVariant` | `inline` · `hero` · `compact` | `hero` seulement ; `inline` et `compact` **jamais** |
| `TemperatureUnit` | `C` · `F` | `C` seulement ; **aucun module ne sert de Fahrenheit** |
| `StackDirection` | `vertical` · `horizontal` | `horizontal` demandé par 2 modules ; `vertical` jamais — c'est le défaut |
| `ChoiceListLayout` | `compact` · `cards` | `compact` demandé par 3 modules ; `cards` jamais côté voyageur — **et de toute façon le livret ignore ce réglage** (§5) |
| `MapInteractionMode` | `pan-zoom` · `none` | **toutes les cartes du livret sont figées** (`none`, 3 modules). Aucune carte manipulable |
| `ImageSize` | `full` · `thumb` | jamais côté voyageur, et **ignoré par le livret** |
| `OverlayPresentation` | `modal` · `bottomSheet` · `fullscreen` | `fullscreen` seulement (5 modules). Le panneau montant et la fenêtre modale existent dans le livret mais **personne ne les demande** |
| `ChartKind` | 4 valeurs | 2 employées côté hôte ; `donut` et `heatmap` **jamais** |

---

## 5. Ce que je proposerais de clarifier

**Rien n'a été modifié.** Ce sont des propositions, chacune argumentée en une phrase. L'ordre va
du plus décisif au plus cosmétique.

### Des rendus qui ne font pas ce que leur nom promet

1. **`QRCode` n'affiche pas un code QR.** Le rendu actuel dessine un motif de 36 cases tiré d'une
   chaîne écrite en dur, identique quel que soit le contenu ; un téléphone ne peut rien en lire.
   `guest-reviews` s'en sert pourtant côté voyageur. À traiter comme une primitive **à
   implémenter**, pas à redessiner.
2. **`WeatherIcon` affiche toujours « ☀ ».** Le code la dit héritée et renvoie vers `Icon` ;
   aucun module ne l'emploie. Proposition : la retirer du catalogue plutôt que la dessiner.
3. **`Toast` n'affiche rien du tout.** Le rendu renvoie systématiquement du vide. Soit c'est un
   oubli, soit c'est un aveu que les messages éphémères sont l'affaire du livret et pas d'un
   module — auquel cas la retirer.
4. **`SectionListItem` affiche deux fois son titre**, une fois en petit gris et une fois en gras,
   parce que le rendu attend deux champs alors que le contrat n'en déclare qu'un. Le rendu ou le
   contrat est en retard sur l'autre.
5. **`BackButton` ne peut rien faire** : le contrat ne lui donne pas d'action, et le rendu en
   cherche une. Soit lui en ajouter une, soit la retirer — le retour arrière étant de toute façon
   l'affaire du livret.
6. **`PullToRefresh` ne rafraîchit rien**, `Tabs` n'a pas d'onglets cliquables,
   `TimeSlotPicker` n'a pas de créneau sélectionnable, `CountdownTimer` ne décompte pas. Quatre
   primitives dont le nom décrit un comportement que le rendu n'a pas. Il vaudrait mieux les
   nommer d'après ce qu'elles montrent, ou les finir.

### Des noms qui trompent

7. **`Text` + `display` fait deux choses sans rapport** : une pastille emoji de 54 px, ou un
   grand nombre — départagées par une expression régulière qui teste si le texte ressemble à une
   température. Proposition : séparer en deux, par exemple une variante « symbole » et une
   variante « grand chiffre », et supprimer la devinette.
8. **`Section` n'a rien à voir avec le module `sections`.** La primitive est un titre suivi de son
   contenu ; le module est l'éditeur de rubriques libres de l'hôte. Deux choses très différentes
   sous le même mot, dans un produit où les deux se croisent.
9. **`Surface` désigne à la fois** une primitive (un bloc avec sa marge), un réglage commun
   (`surface` : niveau de fond) et l'unité que livre un module (« une surface voyageur »). Trois
   sens pour un mot ; c'est celui qui fait trébucher à la lecture du code.
10. **`Modal` et `BottomSheet` ne sont pas la façon d'ouvrir un panneau.** Les panneaux passent
    par une *action*, pas par ces primitives. Garder les deux noms au catalogue entretient
    l'idée qu'on peut poser une fenêtre dans un arbre. Proposition : les retirer, et documenter
    le panneau réel comme l'unique mécanisme.
11. **`paw-print` dessine un cadeau**, `sliders` un engrenage, `gauge` un graphique d'activité.
    Trois noms d'icône qui ne décrivent pas le dessin obtenu.

### Deux primitives qui font presque la même chose

12. **`Badge`, `Pill`, `Tag`, `StatusBadge`** : quatre étiquettes textuelles. `Badge` est une
    pastille teintée ; `Pill` est la même avec un point devant ; `Tag` est un mot gris qui ignore
    `tone` ; `StatusBadge` est une pastille qui affiche « libellé · état » avec l'état non
    traduit. Deux suffiraient : une pastille (avec ou sans point) et un mot discret.
13. **`Stepper` et `FormStepper` ont un rendu identique** — une rangée de segments — et ne
    diffèrent que par le nom de leurs champs (`current` contre `currentStep`). Doublon franc.
14. **`ChoiceList` est rendu exactement comme `RadioGroup`.** Son `layout` (`compact` / `cards`),
    son `emitOnChange`, son action et les icônes et descriptions de ses options sont **tous
    ignorés** par le livret. Trois modules envoient `layout: compact` et cela ne change rien.
    Soit `ChoiceList` reçoit son propre dessin, soit elle est repliée sur `RadioGroup`.
15. **`Notice`, `InlineNotice`, `InfoBanner`** : trois façons de dire quelque chose en passant.
    Seule `InfoBanner` est employée ; `InlineNotice` n'est même pas dessinée ; et le contrat
    d'`InlineNotice` porte **deux** champs pour la même chose (`message` **et** `text`).
16. **`SuccessState`, `CompletionState`** disent la même chose avec deux mises en forme
    différentes, et ni l'une ni l'autre n'est employée.
17. **`LoadingState` et `Spinner`** : la première est la seconde plus un message.
18. **`Icon` porte un jeton d'icône, mais `Text`+`display` porte un emoji, et `ListItem.leading`
    accepte les deux** — avec une expression régulière pour décider lequel. Trois façons de poser
    un pictogramme ; une règle claire manque.

### Des champs jamais renseignés, ou ignorés

19. **Le livret ignore purement et simplement** : `Image.size`, `Chevron.direction`,
    `Skeleton.variant` et `Skeleton.lines`, `ErrorState.retryAction`, `TimeColumn.times`,
    `DateColumn.dates`, `ChoiceList.layout`. Des champs qu'un module peut renseigner sans le
    moindre effet visible.
20. **`TextArea` et `Select` perdent leur valeur initiale**, alors que `TextInput` la conserve.
    Un formulaire pré-rempli ne l'est donc qu'à moitié.
21. **`ColorDotItem` et `Dot` acceptent à la fois `swatch`** (une teinte nommée, du thème) **et
    `color`** (une chaîne libre, n'importe quelle couleur CSS). Le second contourne le thème.
    Proposition : ne garder que `swatch`.
22. **`IconButton` écrit le nom de l'icône en toutes lettres** au lieu de la dessiner, alors que
    `Card`, `ListItem` et `Icon` la résolvent correctement. Incohérence de rendu, pas de contrat.
23. **`TagInput` affiche « Add tag » en anglais en dur**, dans un livret qui est par ailleurs
    entièrement traduit.

### Des jeux de valeurs à resserrer

24. **`Tone` : 9 valeurs, 5 employées, et un comportement qui change entre fond et encre.**
    Le point détaillé est en §4.1. C'est la décision la plus structurante du document : soit on
    aligne chaque ton sur sa propre couleur dans les deux usages, soit on réduit le jeu à ce qui
    est réellement distinguable — à mon sens *neutre, marque, information, succès, attention,
    danger*, en fusionnant `danger` et `emergency` qui partagent déjà leur encre.
25. **`IconName` : 83 noms, 36 employés, 17 glyphes partagés.** Le jeu peut être réduit
    d'environ un tiers sans rien perdre à l'écran, ce qui allégerait d'autant le travail de dessin.
26. **`AnimationKind` n'est demandé par aucun module**, et seul `Stack` est animé. Soit
    l'animation devient une décision du livret et sort du contrat, soit elle est appliquée
    partout — l'état actuel est le pire des deux.
27. **`Swatch` : `white` et `purple` ne servent jamais**, et `white` a besoin d'un liseré pour se
    voir. À confirmer avec les usages réels (bacs de tri, catégories).

### Les habillages qui n'ont rien à faire dans un contrat de module

28. **`TopBar`, `BottomTabBar`, `HeaderTitle`, `Chevron`, `Page`** : l'habillage du livret est
    dessiné par le livret. Aucun module n'en envoie, et un module ne devrait pas pouvoir décider
    de la barre de navigation. Proposition : les sortir du catalogue offert aux modules.

---

## 6. Ce que je n'ai pas pu établir

Ces points sont des **questions ouvertes**, pas des constats.

1. **Les 63 primitives que personne n'emploie sont-elles un catalogue d'avenir ou un reliquat ?**
   Le code ne le dit pas. La réponse change tout : soit il faut les dessiner (elles attendent des
   modules à venir), soit il faut les retirer. Je ne peux pas trancher à la lecture.
2. **`SectionListItem` : est-ce le contrat ou le rendu qui est en retard ?** Le rendu attend
   visiblement un intitulé de groupe *et* un titre ; le contrat n'en déclare qu'un. Lequel des
   deux exprime l'intention, je l'ignore.
3. **`InlineNotice` porte `message` et `text`.** Lequel est le bon, et pourquoi les deux existent,
   je n'ai pas trouvé de trace.
4. **Le `Tone` en fond qui prend la couleur de la marque plutôt que celle du rôle** (`success`,
   `warning`) : je ne sais pas si c'est un choix — garder un livret qui reste à la couleur de
   l'hôte — ou une dérive. Si c'est un choix, il mérite d'être écrit, parce qu'il rend les noms
   `success` et `warning` trompeurs.
5. **Le comportement en mode rangée de `Card`** (dans une liste d'onglets) : je vois comment il
   est déclenché, mais pas à partir de quelle règle produit une surface se retrouve dans une
   liste d'onglets plutôt qu'en cartes pleines.
6. **Je n'ai pas vu le livret tourner.** Tout ce document vient de la lecture du contrat, du code
   de rendu et des arbres d'aperçu livrés. Les rendus que je décris sont ceux que le code produit,
   pas ceux que j'ai observés à l'écran.
7. **Les modules communautaires à venir** pourraient employer n'importe laquelle des 116
   primitives. La liste des 35 décrit l'usage d'aujourd'hui, par les modules officiels ; elle ne
   prédit pas celui de demain.

---

## 7. Sources

| Quoi | Où |
|---|---|
| Les 116 primitives, leurs champs | `portaki-sdk` → `crates/portaki-sdk/sdui_primitives.json` |
| Les variantes et types composés | `portaki-sdk` → `contracts/sdui_types.json` |
| Les réglages communs à toutes | `portaki-sdk` → `crates/portaki-sdk/build.rs` |
| Le rendu vu par le voyageur | `portaki-guest` → `src/features/sdui-renderer-guest/ui/primitives/` |
| La liste des primitives dessinées | `portaki-guest` → `src/features/sdui-renderer-guest/ui/primitive-registry.ts` |
| La traduction ton → couleur | `portaki-guest` → `src/features/sdui-renderer-guest/model/semantic-styles.ts` |
| La traduction nom → dessin d'icône | `portaki-guest` → `src/shared/lib/resolve-sdui-icon.ts` |
| Les couleurs du thème | `portaki-guest` → `src/app/_fsd/styles/globals.css` |
| L'usage réel | `portaki-modules` → `modules/*/src/guest/` et `modules/*/previews.json` |
