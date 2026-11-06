/**
 * Textes de l'interface, en français. Un dictionnaire typé plutôt qu'une bibliothèque i18n
 * (ADR-0006) : une clé absente ou mal orthographiée est une **erreur de compilation**.
 * Pour ajouter une langue : copier ce fichier (`en.ts`) avec les mêmes clés.
 */
export const fr = {
  'app.title': 'TodoQuoi',
  'breadcrumb.home': 'Accueil',
  'tree.empty': 'Aucune tâche. Appuyez sur {shortcut} pour en créer une.',
  'tree.addTask': 'Ajouter une tâche',
  'tree.label': 'Tâches',
  'task.newTitle': 'Nouvelle tâche',
  'task.progress': '{done} sur {total} sous-tâches terminées',
  'task.hasNote': 'Contient une note',
  'status.count': '{total} tâches · {open} ouvertes',
  'status.loading': 'Chargement…',
  'status.detailLater': 'Le panneau de détail arrive à l’incrément I2.',
  'close.withChildren.title': 'Sous-tâches encore ouvertes',
  'close.withChildren.message': '« {title} » a encore {count} sous-tâche(s) ouverte(s).',
  'close.withChildren.all': 'Clôturer tout',
  'close.withChildren.taskOnly': 'Seulement la tâche',
  'close.parent.title': 'Tout est terminé',
  'close.parent.message': 'Toutes les sous-tâches de « {title} » sont terminées. Clôturer « {title} » ?',
  'close.parent.confirm': 'Clôturer',
  'reopen.ancestors.title': 'Rouvrir aussi ?',
  'reopen.ancestors.message': 'Ces tâches parentes sont terminées : {titles}. Les rouvrir aussi ?',
  'reopen.ancestors.confirm': 'Rouvrir',
  'common.cancel': 'Annuler',
  'common.later': 'Plus tard',
  'common.no': 'Non',
  'common.undo': 'Annuler',
  'delete.done': '« {title} » déplacée dans la corbeille',
  'restore.toRoot': 'Tâche restaurée au premier niveau (parent introuvable)',
  'error.prefix': 'Erreur : {message}',
} as const

export type MessageKey = keyof typeof fr
