+++
schema_version = 1
id = "0195f3c4-0000-7000-8000-000000000002"
order = "a1"
title = "Checklist go/no-go"
done = true
done_at = 2026-11-06T09:00:00Z
tags = []
created_at = 2026-11-01T08:00:00Z
updated_at = 2026-11-06T09:00:00Z
updated_by = "pc-bureau"
+++
Checklist de mise en production :

- [x] Tests de non-régression verts
- [ ] Sauvegarde de la base
- [ ] Communication client envoyée

:::radio{label="Environnement cible"}
- ( ) Recette
- (x) Pré-production
- ( ) Production
:::

| Lot | Responsable | État |
| --- | --- | --- |
| Front | Greg | OK |
| API | Léa | En cours |

```sql
UPDATE commande SET statut = 'OUVERTE' WHERE statut = 0;
-- ::audio{src="pas une directive"}
```

> Penser à prévenir le support.
>
> Deuxième paragraphe de la citation.

---

Fin de la checklist.
