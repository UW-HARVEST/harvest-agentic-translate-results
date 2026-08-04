(
(def (list . lst) lst)
(def else True)
(def (not x) (cond (x Nil) (True True)))
(def (cadr x) (car (cdr x)))
(def (cdar x) (cdr (car x)))
(def (caar x) (car (car x)))
(def (cddr x) (cdr (cdr x)))
)
