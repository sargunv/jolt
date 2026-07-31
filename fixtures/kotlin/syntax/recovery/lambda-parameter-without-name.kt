when (element) {
    is KtAnnotatedExpression -> {
        if (elementText.startsWith("/*") && !elementText.endsWith("*/
")) {
            println("doc")
        }
    }
    is KtStringTemplateExpression -> println("template")
}
