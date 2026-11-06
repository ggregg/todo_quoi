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
